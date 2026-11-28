use super::super::outbox::{Outbox, Pushed, MAX_CHAT_BYTES};
use super::*;
use serde_json::Value;

struct Bridge {
    path: PathBuf,
    dir: PathBuf,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Socket paths have a small length limit, so the directory name is short.
fn start(name: &str) -> Bridge {
    let dir = std::env::temp_dir().join(format!("yb-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let (listener, path) = bind(&dir).expect("bind");
    thread::spawn(move || serve(listener, Arc::new(Hub::default())));
    Bridge { path, dir }
}

struct App {
    name: &'static str,
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl App {
    fn raw(bridge: &Bridge, name: &'static str) -> Self {
        let writer = UnixStream::connect(&bridge.path).expect("connect");
        writer
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        let reader = BufReader::new(writer.try_clone().expect("clone"));
        Self {
            name,
            reader,
            writer,
        }
    }

    fn join(bridge: &Bridge, name: &'static str) -> Self {
        let mut app = Self::raw(bridge, name);
        app.say(&json!({"app": name, "app_version": "1.0.0", "protocols": [1]}));
        let welcome = app.hear().expect("welcome");
        assert_eq!(welcome["type"], "welcome");
        assert_eq!(welcome["runtime_version"], env!("CARGO_PKG_VERSION"));
        app
    }

    fn say(&mut self, value: &Value) {
        send(&mut self.writer, &value.to_string()).expect("send");
    }

    fn publish(&mut self, stream: &str, kind: &str, text: &str) {
        let name = self.name;
        self.say(&json!({"v": 1, "id": "m", "stream": stream, "type": kind,
            "origin": name, "object": "o", "body": {"text": text}}));
    }

    fn hear(&mut self) -> Option<Value> {
        read_line(&mut self.reader)
            .ok()
            .flatten()
            .map(|l| serde_json::from_str(&l).expect("json"))
    }
}

#[test]
fn a_change_reaches_the_other_app_and_not_its_sender() {
    let bridge = start("fan");
    let mut studio = App::join(&bridge, "studio");
    let mut chat = App::join(&bridge, "chat");
    studio.publish("chat", "message.append", "from studio");
    assert_eq!(
        chat.hear().expect("delivered")["body"]["text"],
        "from studio"
    );
    chat.publish("chat", "message.append", "from chat");
    // The first thing studio hears is the other app's message, not its own.
    assert_eq!(
        studio.hear().expect("delivered")["body"]["text"],
        "from chat"
    );
}

#[test]
fn the_socket_is_private_to_the_user() {
    let bridge = start("perm");
    let mode = |path: &Path| fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
    assert_eq!(mode(&bridge.dir), 0o700);
    assert_eq!(mode(&bridge.path), 0o600);
}

#[test]
fn a_second_bridge_does_not_take_over() {
    let bridge = start("two");
    let error = bind(&bridge.dir).expect_err("already serving");
    assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
}

#[test]
fn a_leftover_socket_file_is_replaced() {
    let dir = std::env::temp_dir().join(format!("yb-old-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    drop(bind(&dir).expect("first"));
    assert!(bind(&dir).is_ok());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_app_with_another_protocol_is_told_which_side_to_update() {
    let bridge = start("ver");
    let mut app = App::raw(&bridge, "future");
    app.say(&json!({"app": "future", "app_version": "9", "protocols": [2]}));
    let reply = app.hear().expect("error reply");
    assert!(
        reply["message"]
            .as_str()
            .expect("text")
            .contains("update yana-rt"),
        "{reply}"
    );
    assert!(app.hear().is_none());
}

#[test]
fn control_messages_and_borrowed_names_are_refused() {
    let bridge = start("ref");
    let mut watcher = App::join(&bridge, "watcher");
    let mut intruder = App::join(&bridge, "intruder");
    intruder.publish("control", "approval.decision", "allow");
    assert_eq!(intruder.hear().expect("refused")["type"], "error");
    let mut liar = App::join(&bridge, "liar");
    liar.say(
        &json!({"v": 1, "id": "m", "stream": "state", "type": "agent",
        "origin": "studio", "body": {}}),
    );
    assert_eq!(liar.hear().expect("refused")["type"], "error");
    // Nothing refused ever reached another app: the next line is the honest one.
    let mut honest = App::join(&bridge, "honest");
    honest.publish("state", "agent", "running");
    assert_eq!(watcher.hear().expect("delivered")["origin"], "honest");
}

#[test]
fn a_message_before_hello_and_an_oversized_line_close_the_connection() {
    let bridge = start("bad");
    let mut early = App::raw(&bridge, "early");
    early.publish("state", "agent", "no hello");
    assert_eq!(early.hear().expect("error")["type"], "error");
    let mut huge = App::raw(&bridge, "huge");
    // The server may close while we are still writing.
    let _ = huge.writer.write_all(&vec![b'a'; MAX_LINE_BYTES + 2]);
    assert!(huge.hear().is_none());
}

fn envelope(stream: &str, kind: &str) -> protocol::Envelope {
    let line = json!({"v": 1, "id": "m", "stream": stream, "type": kind, "origin": "a"});
    protocol::parse_envelope(&line.to_string()).expect("valid")
}

#[test]
fn a_slow_reader_keeps_only_the_newest_state_and_gets_it_before_chat() {
    let outbox = Outbox::default();
    outbox.push(&envelope("chat", "message"), "chat-1");
    outbox.push(&envelope("state", "agent"), "old");
    outbox.push(&envelope("state", "agent"), "new");
    outbox.push(&envelope("chat", "message"), "chat-2");
    let heard: Vec<_> = (0..3).filter_map(|_| outbox.next()).collect();
    assert_eq!(heard, ["new", "chat-1", "chat-2"]);
}

#[test]
fn too_much_unread_chat_cuts_the_client_off() {
    let outbox = Outbox::default();
    let big = "a".repeat(MAX_CHAT_BYTES);
    assert_eq!(
        outbox.push(&envelope("chat", "message"), &big),
        Pushed::Accepted
    );
    assert_eq!(
        outbox.push(&envelope("chat", "message"), "x"),
        Pushed::Overflow
    );
    assert_eq!(
        outbox.push(&envelope("state", "agent"), "x"),
        Pushed::Closed
    );
    assert_eq!(outbox.next(), None);
}
