//! Who is connected, and delivery to everyone but the sender.

use super::outbox::Outbox;
use super::protocol::Envelope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

pub const MAX_CLIENTS: usize = 32;

struct Client {
    app: String,
    outbox: Arc<Outbox>,
}

#[derive(Default)]
struct Clients {
    next_id: u64,
    connected: HashMap<u64, Client>,
}

#[derive(Default)]
pub struct Hub {
    clients: Mutex<Clients>,
}

impl Hub {
    fn lock(&self) -> MutexGuard<'_, Clients> {
        self.clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// `None` when the bridge is full.
    pub fn join(&self, app: &str) -> Option<(u64, Arc<Outbox>)> {
        let mut clients = self.lock();
        if clients.connected.len() >= MAX_CLIENTS {
            return None;
        }
        clients.next_id += 1;
        let id = clients.next_id;
        let outbox = Arc::new(Outbox::default());
        let client = Client {
            app: app.to_string(),
            outbox: Arc::clone(&outbox),
        };
        clients.connected.insert(id, client);
        Some((id, outbox))
    }

    pub fn leave(&self, id: u64) {
        if let Some(client) = self.lock().connected.remove(&id) {
            client.outbox.close();
        }
    }

    /// Hands the line to every other app. Never waits for any of them, and
    /// never sends a change back to the app that made it.
    pub fn publish(&self, sender: u64, envelope: &Envelope, line: &str) {
        let clients = self.lock();
        for (id, client) in &clients.connected {
            if *id != sender && client.app != envelope.origin {
                // An overflowing client closes its own outbox; its threads
                // then disconnect it and it reloads on the next connection.
                let _ = client.outbox.push(envelope, line);
            }
        }
    }
}
