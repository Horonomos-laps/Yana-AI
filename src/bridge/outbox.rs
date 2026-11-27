//! What waits to be written to one client.
//!
//! There is no queue shared between clients: a slow reader only affects its
//! own outbox. Values that replace each other (`state`, `settings`, `system`)
//! keep the newest only; `chat` keeps order and has a hard size limit.

use super::protocol::{Envelope, Stream};
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard};

/// Chat text a client may fall behind by before it is cut off and must reload.
pub const MAX_CHAT_BYTES: usize = 4 * 1024 * 1024;
/// Distinct replaceable values (one per stream, type and object).
pub const MAX_LATEST_ENTRIES: usize = 1024;

struct Line {
    stream: Stream,
    key: (String, Option<String>),
    text: String,
}

#[derive(Default)]
struct Waiting {
    latest: Vec<Line>,
    chat: VecDeque<String>,
    chat_bytes: usize,
    closed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Pushed {
    Accepted,
    /// The client is too far behind: the outbox is now closed.
    Overflow,
    Closed,
}

#[derive(Default)]
pub struct Outbox {
    waiting: Mutex<Waiting>,
    ready: Condvar,
}

impl Outbox {
    fn lock(&self) -> MutexGuard<'_, Waiting> {
        // A panic while holding the lock leaves plain data, still safe to use.
        self.waiting
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Never blocks on the client.
    pub fn push(&self, envelope: &Envelope, text: &str) -> Pushed {
        let mut waiting = self.lock();
        if waiting.closed {
            return Pushed::Closed;
        }
        let fits = if envelope.stream == Stream::Chat {
            waiting.chat_bytes += text.len();
            waiting.chat.push_back(text.to_string());
            waiting.chat_bytes <= MAX_CHAT_BYTES
        } else {
            replace_latest(&mut waiting.latest, envelope, text)
        };
        if !fits {
            *waiting = Waiting {
                closed: true,
                ..Waiting::default()
            };
        }
        drop(waiting);
        self.ready.notify_all();
        if fits {
            Pushed::Accepted
        } else {
            Pushed::Overflow
        }
    }

    /// Waits for the next line. Replaceable values go before chat text.
    /// Returns `None` once the outbox is closed.
    pub fn next(&self) -> Option<String> {
        let mut waiting = self.lock();
        loop {
            if waiting.closed {
                return None;
            }
            if !waiting.latest.is_empty() {
                return Some(waiting.latest.remove(0).text);
            }
            if let Some(text) = waiting.chat.pop_front() {
                waiting.chat_bytes -= text.len();
                return Some(text);
            }
            waiting = self
                .ready
                .wait(waiting)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    pub fn close(&self) {
        self.lock().closed = true;
        self.ready.notify_all();
    }
}

fn replace_latest(latest: &mut Vec<Line>, envelope: &Envelope, text: &str) -> bool {
    let key = (envelope.kind.clone(), envelope.object.clone());
    let stream = envelope.stream;
    if let Some(line) = latest
        .iter_mut()
        .find(|line| line.stream == stream && line.key == key)
    {
        line.text = text.to_string();
        return true;
    }
    latest.push(Line {
        stream,
        key,
        text: text.to_string(),
    });
    latest.len() <= MAX_LATEST_ENTRIES
}
