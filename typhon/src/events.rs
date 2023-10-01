use typhon_types::Event;

use futures_core::stream::Stream;
use tokio::sync::mpsc::*;

pub struct EventLogger {
    senders: Vec<Sender<Event>>,
    shutdown: bool,
}

impl EventLogger {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
            shutdown: false,
        }
    }

    pub async fn log(&mut self, e: Event) {
        let mut senders: Vec<Sender<Event>> = Vec::new();
        for sender in self.senders.drain(..) {
            match sender.send(e.clone()).await {
                Ok(()) => senders.push(sender),
                Err(_) => (),
            };
        }
        self.senders = senders;
    }

    pub fn listen(&mut self) -> Option<impl Stream<Item = Event>> {
        if self.shutdown {
            None
        } else {
            let (sender, mut receiver) = channel(256);
            self.senders.push(sender);
            Some(async_stream::stream! {
                while let Some(e) = receiver.recv().await {
                    yield e;
                }
            })
        }
    }

    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.senders.clear()
    }
}
