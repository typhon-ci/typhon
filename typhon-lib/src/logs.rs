pub mod live {
    use crate::RUNTIME;

    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio::sync::watch;

    use std::collections::HashMap;

    #[derive(Debug)]
    enum Msg<Id> {
        Remove {
            id: Id,
            dump_sender: oneshot::Sender<Option<String>>,
        },
        Init {
            id: Id,
        },
        Line {
            id: Id,
            line: String,
        },
        Listen {
            id: Id,
            lines_sender: mpsc::UnboundedSender<String>,
            not_found_sender: oneshot::Sender<bool>,
        },
        Shutdown,
    }

    #[derive(Debug)]
    pub struct Cache<Id> {
        sender: mpsc::UnboundedSender<Msg<Id>>,
        watch: watch::Receiver<()>,
    }

    impl<Id: Clone + Eq + PartialEq + Send + std::fmt::Debug + std::hash::Hash + 'static> Cache<Id>
    where
        for<'a> &'a Id: Send,
    {
        pub fn new() -> Self {
            let (sender, mut receiver) = mpsc::unbounded_channel();
            let (watch_send, watch) = watch::channel(());
            RUNTIME.spawn(async move {
                type Listeners = Vec<mpsc::UnboundedSender<String>>;
                let mut state: HashMap<Id, (Vec<String>, Listeners)> = HashMap::new();
                while let Some(msg) = receiver.recv().await {
                    match msg {
                        Msg::Remove { id, dump_sender } => {
                            dump_sender
                                .send(state.remove(&id).map(|(lines, _)| lines.join("\n")))
                                .unwrap();
                        }
                        Msg::Init { id } => {
                            state.insert(id.clone(), (Vec::new(), Vec::new()));
                        }
                        Msg::Line { id, line } => {
                            let (lines, ref mut listeners) = state
                                .get_mut(&id)
                                .expect("log channels need to be initialized before sending lines");
                            lines.push(line.clone());

                            let mut new_listeners: Listeners = Vec::new();
                            for listener in listeners.drain(..) {
                                match listener.send(line.clone()) {
                                    Ok(()) => new_listeners.push(listener),
                                    Err(_) => (),
                                }
                            }
                            *listeners = new_listeners;
                        }
                        Msg::Listen {
                            id,
                            lines_sender,
                            not_found_sender,
                        } => {
                            if let Some((lines, listeners)) = state.get_mut(&id) {
                                not_found_sender.send(false).unwrap();
                                for line in lines {
                                    lines_sender.send(line.clone()).unwrap();
                                }
                                listeners.push(lines_sender);
                            } else {
                                not_found_sender.send(true).unwrap();
                            }
                        }
                        Msg::Shutdown => break,
                    }
                }
                let _watch_send = watch_send;
            });
            Self { sender, watch }
        }

        pub fn remove(&self, id: &Id) -> Option<String> {
            let (dump_sender, remove_receiver) = oneshot::channel();
            self.sender
                .send(Msg::Remove {
                    id: id.clone(),
                    dump_sender,
                })
                .unwrap();
            remove_receiver.blocking_recv().unwrap()
        }

        pub fn init(&self, id: &Id) -> () {
            self.sender.send(Msg::Init { id: id.clone() }).unwrap();
        }

        pub fn listen(&self, id: &Id) -> Option<impl futures_core::stream::Stream<Item = String>> {
            let (lines_sender, mut lines_receiver) = mpsc::unbounded_channel();
            let (not_found_sender, not_found_receiver) = oneshot::channel();
            self.sender
                .send(Msg::Listen {
                    id: id.clone(),
                    lines_sender,
                    not_found_sender,
                })
                .unwrap();

            if not_found_receiver.blocking_recv().unwrap() {
                None
            } else {
                Some(async_stream::stream! {
                    while let Some(i) = lines_receiver.recv().await {
                        yield i;
                    }
                })
            }
        }

        pub fn send_line(&self, id: &Id, line: String) {
            self.sender
                .send(Msg::Line {
                    id: id.clone(),
                    line,
                })
                .unwrap()
        }

        pub async fn shutdown(&self) {
            let _ = self.sender.send(Msg::Shutdown);
            while self.watch.clone().changed().await.is_ok() {}
        }
    }
}
