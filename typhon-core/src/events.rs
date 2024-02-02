use crate::RUNTIME;

use typhon_types::Event;

use futures_core::stream::Stream;
use tokio::sync::mpsc;
use tokio::sync::watch;

pub enum Msg {
    Emit(Event),
    Listen(mpsc::UnboundedSender<Event>),
    Shutdown,
}

pub struct EventLogger {
    sender: mpsc::UnboundedSender<Msg>,
    watch: watch::Receiver<()>,
}

impl EventLogger {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let (watch_send, watch) = watch::channel(());
        RUNTIME.spawn(async move {
            let mut senders: Vec<mpsc::UnboundedSender<Event>> = Vec::new();
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Msg::Emit(event) => {
                        let mut new_senders: Vec<mpsc::UnboundedSender<Event>> = Vec::new();
                        for sender in senders.drain(..) {
                            match sender.send(event.clone()) {
                                Ok(()) => new_senders.push(sender),
                                Err(_) => (),
                            }
                        }
                        senders = new_senders;
                    }
                    Msg::Listen(sender) => senders.push(sender),
                    Msg::Shutdown => break,
                }
            }
            let _watch_send = watch_send;
        });
        Self { sender, watch }
    }

    pub fn log(&self, event: Event) {
        let _ = self.sender.send(Msg::Emit(event));
    }

    pub fn listen(&self) -> Option<impl Stream<Item = Event>> {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let _ = self.sender.send(Msg::Listen(sender));
        Some(async_stream::stream! {
            yield Event::Ping;
            while let Some(e) = receiver.recv().await {
                yield e;
            }
        })
    }

    pub async fn shutdown(&self) {
        let _ = self.sender.send(Msg::Shutdown);
        while self.watch.clone().changed().await.is_ok() {}
    }
}
