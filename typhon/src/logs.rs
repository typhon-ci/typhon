pub mod live {
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    #[derive(Debug)]
    enum Msg<Id> {
        Reset {
            id: Id,
        },
        Line {
            id: Id,
            line: String,
        },
        Listen {
            id: Id,
            lines_sender: mpsc::Sender<String>,
            not_found_sender: oneshot::Sender<bool>,
        },
        Shutdown,
    }

    #[derive(Debug)]
    pub struct Cache<Id> {
        sender: mpsc::Sender<Msg<Id>>,
        handle: Mutex<Option<JoinHandle<()>>>,
    }

    impl<Id: Clone + Eq + PartialEq + Send + std::fmt::Debug + std::hash::Hash + 'static> Cache<Id>
    where
        for<'a> &'a Id: Send,
    {
        pub fn new() -> Self {
            let (sender, mut receiver) = mpsc::channel(32);
            let handle = tokio::spawn(async move {
                type Listeners = Vec<mpsc::Sender<String>>;
                let mut state: HashMap<Id, (Vec<String>, Listeners)> = HashMap::new();
                while let Some(msg) = receiver.recv().await {
                    match msg {
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
                                match listener.send(line.clone()).await {
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
                                    lines_sender.send(line.clone()).await.unwrap();
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

        pub async fn listen(
            &self,
            id: &Id,
        ) -> Option<impl futures_core::stream::Stream<Item = String>> {
            let (lines_sender, mut lines_receiver) = mpsc::channel(32);
            let (not_found_sender, not_found_receiver) = oneshot::channel();
            self.sender
                .send(Msg::Listen {
                    id: id.clone(),
                    lines_sender,
                    not_found_sender,
                })
                .await
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

        pub async fn send_line(&self, id: &Id, line: String) {
            self.sender
                .send(Msg::Line {
                    id: id.clone(),
                    line,
                })
                .await
                .unwrap()
        }

        pub async fn reset(&self, id: &Id) {
            self.sender
                .send(Msg::Reset { id: id.clone() })
                .await
                .unwrap()
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
}
