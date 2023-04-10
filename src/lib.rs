use std::io::{BufRead, StdoutLock, Write};
use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message<Payload> {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body<Payload>,
}

impl<Payload> Message<Payload> {
    pub fn into_reply(self, id: Option<&mut usize>) -> Self {
        Self {
            src: self.dst,
            dst: self.src,
            body: Body {
                id: id.map(|id| {
                    let mid = *id;
                    *id += 1;
                    mid
                }),
                in_reply_to: self.body.id,
                payload: self.body.payload,
            },
        }
    }

    pub fn send(&self, output: &mut impl Write) -> Result<()>
        where
            Payload: Serialize
    {
        serde_json::to_writer(&mut *output, self).context("serialize response to topology")?;
        output.write_all(b"\n").context("write trailing newline")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event<Payload, InjectedPayload = ()> {
    Message(Message<Payload>),
    Injected(InjectedPayload),
    EOF,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Body<Payload> {
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InitPayload {
    Init(Init),
    InitOk,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<S, Payload, InjectedPayload = ()> {
    fn from_init(state: S, init: Init, tx: Sender<Event<Payload, InjectedPayload>>) -> Result<Self> where Self: Sized;
    fn step(&mut self, input: Event<Payload, InjectedPayload>, output: &mut StdoutLock) -> Result<()>;
}


pub fn main_loop<S, N, P, IP>(init_state: S) -> Result<()>
    where
        P: DeserializeOwned + Send + 'static,
        N: Node<S, P, IP>,
        IP: Send + 'static
{
    let (tx, rx) = std::sync::mpsc::channel();

    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    let mut stdout = std::io::stdout().lock();

    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin.next().expect("no init message recv").context("failed to read init message from stdin")?)
        .context("init message could not be deserialized")?;

    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("first message should be init")
    };

    let mut node = N::from_init(init_state, init, tx.clone()).context("not init failed")?;


    let reply = Message {
        src: init_msg.dst,
        dst: init_msg.src,
        body: Body {
            id: Some(0),
            in_reply_to: init_msg.body.id,
            payload: InitPayload::InitOk,
        },
    };

    serde_json::to_writer(&mut stdout, &reply).context("serialize response to init")?;
    stdout.write_all(b"\n").context("write trailing newline")?;


    drop(stdin);

    let jh = std::thread::spawn(move || {
        let stdin = std::io::stdin().lock();
        let stdin = stdin.lines();
        for line in stdin {
            let line = line.context("Maelstorm input from STDIN could not be read")?;
            let input: Message<P> = serde_json::from_str(&line).context("Maelstorm input from STDIN could not be deserialized")?;

            if let Err(_) = tx.send(Event::Message(input)) {
                return Ok::<_, anyhow::Error>(());
            }
        }
        let _ = tx.send(Event::EOF);
        Ok(())
    });

    for input in rx {
        node.step(input, &mut stdout).context("Node step function failed")?;
    }

    jh.join().expect("stdin thread paniced").context("stdin thread err");

    Ok(())
}