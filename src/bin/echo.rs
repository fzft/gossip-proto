use std::io::{StdoutLock, Write};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use naruto::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Echo { echo: String },
    EchoOk { echo: String },
}

struct EchoNode {
    id: usize,
}

impl Node<(), Payload> for EchoNode {
    fn from_init(_state: (), _init: Init) -> Result<Self> where Self: Sized {
        Ok(EchoNode { id: 1 })
    }

    fn step(
        &mut self,
        input: Message<Payload>,
        output: &mut StdoutLock)
        -> Result<()> {
        let mut reply = input.into_reply(Some(&mut self.id));
        match reply.body.payload {
            Payload::Echo { echo } => {
                reply.body.payload = Payload::EchoOk {
                    echo };
                serde_json::to_writer(&mut *output, &reply).context("serialize response to echo")?;
                output.write_all(b"\n").context("write trailing newline")?;
            }
            Payload::EchoOk { .. } => {},
        }

        Ok(())
    }
}


fn main() -> Result<()> {

    main_loop::<_, EchoNode, _>(())
}
