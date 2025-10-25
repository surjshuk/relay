use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering}
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Room {
    tx: broadcast::Sender<String>,
    users: Arc<AtomicUsize>
}

impl Room {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel::<String>(capacity);

        Self {
            tx,
            users: Arc::new(AtomicUsize::new(0))
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    pub fn send<S: Into<String>>(&self, msg: S) {
        let _ = self.tx.send(msg.into());
    }

    pub fn inc(&self) {
        self.users.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.users.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn len(&self) -> usize {
        self.users.load(Ordering::Relaxed)
    }
}