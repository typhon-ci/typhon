use typhon_types::Event;

use futures_core::stream::Stream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::block_in_place;
use tokio::task::JoinHandle;

pub enum Msg {
    Emit(Event),
    Listen(mpsc::Sender<Event>),
    Shutdown,
}

pub struct EventLogger {
    sender: mpsc::Sender<Msg>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl EventLogger {
    pub fn new() -> Self {
        use Msg::*;
        let (sender, mut receiver) = mpsc::channel(256);
        let handle = tokio::spawn(async move {
            let mut senders: Vec<mpsc::Sender<Event>> = Vec::new();
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Emit(event) => {
                        let mut new_senders: Vec<mpsc::Sender<Event>> = Vec::new();
                        for sender in senders.drain(..) {
                            match sender.send(event.clone()).await {
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
        let _ = self.sender.try_send(Msg::Emit(event));
    }

    pub fn listen(&self) -> Option<impl Stream<Item = Event>> {
        let (sender, mut receiver) = mpsc::channel(256);
        let _ = self.sender.try_send(Msg::Listen(sender));
        Some(async_stream::stream! {
            while let Some(e) = block_in_place(|| receiver.blocking_recv()) {
                yield e;
            }
        })
    }

    pub async fn shutdown(&self) {
        let handle = self.handle.lock().await.take();
        if let Some(handle) = handle {
            if self.sender.send(Msg::Shutdown).await.is_ok() {
                let _ = handle.await;
            } else {
                handle.abort();
            }
        }
    }
}
