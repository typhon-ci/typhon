pub mod live {
    use crate::RUNTIME;

    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    use std::collections::HashMap;

    #[derive(Debug)]
    enum Msg<Id> {
        Dump {
            id: Id,
            dump_sender: oneshot::Sender<String>,
            not_found_sender: oneshot::Sender<bool>,
        },
        Reset {
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
        handle: Mutex<Option<JoinHandle<()>>>,
    }

    impl<Id: Clone + Eq + PartialEq + Send + std::fmt::Debug + std::hash::Hash + 'static> Cache<Id>
    where
        for<'a> &'a Id: Send,
    {
        pub fn new() -> Self {
            let (sender, mut receiver) = mpsc::unbounded_channel();
            let handle = RUNTIME.spawn(async move {
                type Listeners = Vec<mpsc::UnboundedSender<String>>;
                let mut state: HashMap<Id, (Vec<String>, Listeners)> = HashMap::new();
                while let Some(msg) = receiver.recv().await {
                    match msg {
                        Msg::Dump {
                            id,
                            dump_sender,
                            not_found_sender,
                        } => {
                            if let Some((lines, _listeners)) = state.get_mut(&id) {
                                not_found_sender.send(false).unwrap();
                                let mut dump: Vec<String> = Vec::new();
                                for line in lines {
                                    dump.push(line.to_string());
                                }
                                dump_sender.send(dump.join("\n")).unwrap();
                            } else {
                                not_found_sender.send(true).unwrap();
                                drop(dump_sender);
                            }
                        }
                        Msg::Reset { id } => {
                            state.remove(&id);
                        }
                        Msg::Line { id, line } => {
                            if !state.contains_key(&id) {
                                state.insert(id.clone(), (vec![], Vec::new()));
                            }
                            let (lines, ref mut listeners) = state.get_mut(&id).unwrap();
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
                                drop(lines_sender)
                            }
                        }
                        Msg::Shutdown => break,
                    }
                }
            });
            Self {
                sender,
                handle: Mutex::new(Some(handle)),
            }
        }

        pub fn dump(&self, id: &Id) -> Option<String> {
            let (dump_sender, dump_receiver) = oneshot::channel();
            let (not_found_sender, not_found_receiver) = oneshot::channel();
            self.sender
                .send(Msg::Dump {
                    id: id.clone(),
                    dump_sender,
                    not_found_sender,
                })
                .unwrap();

            if not_found_receiver.blocking_recv().unwrap() {
                None
            } else {
                Some(dump_receiver.blocking_recv().unwrap())
            }
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

        pub async fn listen_async(
            &self,
            id: &Id,
        ) -> Option<impl futures_core::stream::Stream<Item = String>> {
            let (lines_sender, mut lines_receiver) = mpsc::unbounded_channel();
            let (not_found_sender, not_found_receiver) = oneshot::channel();
            self.sender
                .send(Msg::Listen {
                    id: id.clone(),
                    lines_sender,
                    not_found_sender,
                })
                .unwrap();

            if not_found_receiver.await.unwrap() {
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

        pub fn reset(&self, id: &Id) {
            self.sender.send(Msg::Reset { id: id.clone() }).unwrap()
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
}
