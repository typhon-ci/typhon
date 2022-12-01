use typhon_types::*;

use actix::prelude::*;
use actix_web_actors::ws;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Msg(Event);

pub struct Session {
    hb: Instant,
}

impl Session {
    pub fn new() -> Self {
        Self { hb: Instant::now() }
    }

    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"hi");
        });
    }
}

impl Actor for Session {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        let addr = ctx.address();
        let _ = tokio::spawn(async move {
            let listeners = &mut *crate::LISTENERS.get().unwrap().lock().await;
            listeners.sessions.insert(addr);
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let addr = ctx.address();
        let _ = tokio::spawn(async move {
            let ref mut listeners = crate::LISTENERS.get().unwrap().lock().await;
            let _ = listeners.sessions.remove(&addr);
        });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Session {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            _ => (),
        }
    }
}

impl Handler<Msg> for Session {
    type Result = ();

    fn handle(&mut self, msg: Msg, ctx: &mut Self::Context) {
        let Msg(e) = msg;
        ctx.text(serde_json::to_string(&e).expect("failed to serialize event"));
    }
}

pub struct Listeners {
    sessions: HashSet<Addr<Session>>,
}

impl Listeners {
    pub fn new() -> Self {
        Self {
            sessions: HashSet::new(),
        }
    }

    pub fn log(&self, e: Event) {
        let _ = self
            .sessions
            .iter()
            .for_each(|addr| addr.do_send(Msg(e.clone())));
    }
}
