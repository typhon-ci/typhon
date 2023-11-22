use crate::RUNTIME;

use typhon_types::Event;

use futures_core::stream::Stream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub enum Msg {
    Emit(Event),
    Listen(mpsc::UnboundedSender<Event>),
    Shutdown,
}

pub struct EventLogger {
    sender: mpsc::UnboundedSender<Msg>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl EventLogger {
    pub fn new() -> Self {
        use Msg::*;
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = RUNTIME.spawn(async move {
            let mut senders: Vec<mpsc::UnboundedSender<Event>> = Vec::new();
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Emit(event) => {
                        let mut new_senders: Vec<mpsc::UnboundedSender<Event>> = Vec::new();
                        for sender in senders.drain(..) {
                            match sender.send(event.clone()) {
                                Ok(()) => new_senders.push(sender),
                                Err(_) => (),
                            }
                        }
                        senders = new_senders;
                    }
                    Listen(sender) => senders.push(sender),
                    Shutdown => break,
                }
            }
        });
        Self {
            sender,
            handle: Mutex::new(Some(handle)),
        }
    }

    pub fn log(&self, event: Event) {
        let _ = self.sender.send(Msg::Emit(event));
    }

    pub fn listen(&self) -> Option<impl Stream<Item = Event>> {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let _ = self.sender.send(Msg::Listen(sender));
        Some(async_stream::stream! {
            while let Some(e) = receiver.recv().await {
                yield e;
            }
        })
    }

    pub async fn shutdown(&self) {
        let handle = self.handle.lock().await.take();
        if let Some(handle) = handle {
            if self.sender.send(Msg::Shutdown).is_ok() {
                let _ = handle.await;
            } else {
                handle.abort();
            }
        }
    }
}
