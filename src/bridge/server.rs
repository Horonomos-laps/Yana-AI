//! The local socket of the bridge (Unix only for now).
//!
//! Who may connect: processes of the same user, checked on the socket peer, on
//! a socket inside a directory only that user can open. That does not prove
//! which app is connecting, so the `control` stream (stop, approvals) is
//! refused until per-app tokens exist (SPEC section 5).

use super::hub::Hub;
use super::protocol::{self, Stream, MAX_LINE_BYTES, PROTOCOL_VERSION};
use serde_json::json;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const SOCKET_NAME: &str = "bridge.sock";
/// A client must say hello quickly, so idle strangers do not hold threads.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// A client that stops reading is dropped instead of blocking its writer forever.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// Creates the private directory and binds the socket. Refuses to take over a
/// socket another bridge is serving.
pub fn bind(dir: &Path) -> io::Result<(UnixListener, PathBuf)> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    let path = dir.join(SOCKET_NAME);
    if fs::symlink_metadata(&path).is_ok() {
        if UnixStream::connect(&path).is_ok() {
            let busy = "a bridge is already serving this directory";
            return Err(io::Error::new(io::ErrorKind::AddrInUse, busy));
        }
        fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok((listener, path))
}

/// Serves until the listener fails. Each client gets its own threads.
pub fn serve(listener: UnixListener, hub: Arc<Hub>) -> io::Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        let hub = Arc::clone(&hub);
        thread::spawn(move || {
            if let Err(error) = client(stream, &hub) {
                eprintln!("yana-rt bridge: client closed: {error}");
            }
        });
    }
    Ok(())
}

fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let fd = stream.as_raw_fd();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut credentials = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: `fd` is a live socket and `size` matches the struct passed.
        let result = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut size,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(credentials.uid)
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let (mut uid, mut gid) = (0, 0);
        // SAFETY: `fd` is a live socket and both pointers are valid locals.
        if unsafe { libc::getpeereid(fd, &mut uid, &mut gid) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(uid)
    }
}

/// Reads one line, never buffering more than the protocol allows.
/// `Ok(None)` means the client closed the connection.
fn read_line(reader: &mut BufReader<UnixStream>) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let limit = MAX_LINE_BYTES as u64 + 1;
    reader.by_ref().take(limit).read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.last() != Some(&b'\n') {
        return Err(invalid("line is too long or was cut off"));
    }
    bytes.pop();
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| invalid("line is not UTF-8"))
}

fn invalid(detail: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail.to_string())
}

fn send(stream: &mut UnixStream, line: &str) -> io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")
}

fn refuse(stream: &mut UnixStream, reason: &str) -> io::Error {
    // Best effort: the client may already be gone.
    let _ = send(
        stream,
        &json!({"type": "error", "message": reason}).to_string(),
    );
    invalid(reason)
}

fn client(mut stream: UnixStream, hub: &Hub) -> io::Result<()> {
    // SAFETY: getuid has no preconditions and cannot fail.
    if peer_uid(&stream)? != unsafe { libc::getuid() } {
        return Err(refuse(&mut stream, "the bridge only serves its own user"));
    }
    stream.set_read_timeout(Some(HELLO_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let Some(first) = read_line(&mut reader)? else {
        return Ok(());
    };
    let hello = protocol::parse_hello(&first)
        .and_then(|hello| protocol::negotiate(&[PROTOCOL_VERSION], &hello).map(|v| (hello, v)));
    let (hello, version) = match hello {
        Ok(accepted) => accepted,
        Err(error) => return Err(refuse(&mut stream, &error.to_string())),
    };
    let Some((id, outbox)) = hub.join(&hello.app) else {
        return Err(refuse(&mut stream, "too many clients"));
    };
    let welcome = json!({
        "type": "welcome", "protocol": version, "runtime_version": env!("CARGO_PKG_VERSION")
    });
    let result = send(&mut stream, &welcome.to_string()).and_then(|()| {
        stream.set_read_timeout(None)?;
        let mut writer = stream.try_clone()?;
        let pending = Arc::clone(&outbox);
        thread::spawn(move || {
            while let Some(line) = pending.next() {
                if send(&mut writer, &line).is_err() {
                    break;
                }
            }
            // Wakes the reader so a cut-off client is removed from the hub.
            let _ = writer.shutdown(std::net::Shutdown::Both);
        });
        relay(&mut reader, &mut stream, hub, (id, &hello.app))
    });
    hub.leave(id);
    result
}

fn relay(
    reader: &mut BufReader<UnixStream>,
    stream: &mut UnixStream,
    hub: &Hub,
    sender: (u64, &str),
) -> io::Result<()> {
    while let Some(line) = read_line(reader)? {
        let envelope = match protocol::parse_envelope(&line) {
            Ok(envelope) => envelope,
            Err(error) => return Err(refuse(stream, &error.to_string())),
        };
        if envelope.origin != sender.1 {
            return Err(refuse(
                stream,
                "origin does not match the name given at hello",
            ));
        }
        if envelope.stream == Stream::Control {
            return Err(refuse(stream, "control messages are not accepted yet"));
        }
        hub.publish(sender.0, &envelope, &line);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
