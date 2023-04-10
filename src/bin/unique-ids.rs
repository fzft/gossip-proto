use std::io::{StdoutLock, Write};
use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use naruto::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Generate,
    GenerateOk {
        #[serde(rename = "id")]
        guid: String
    },
}

struct UniqueNode {
    node: String,
    id: usize,
}

impl Node<(), Payload> for UniqueNode {
    fn from_init(_state: (), init: Init, _tx: Sender<Event<Payload>> ) -> Result<Self> where Self: Sized {
        Ok(UniqueNode {
            node: init.node_id,
            id: 1,
        })
    }

    fn step(
        &mut self,
        input: Event<Payload>,
        output: &mut StdoutLock)
        -> Result<()> {
        
        let Event::Message(input) = input else {
            panic!("got injected event when there's no event injection")
        };
        let guid= format!("{}-{}", self.node, self.id);
        let mut reply = input.into_reply(Some(&mut self.id));
        match reply.body.payload {
            Payload::Generate => {
                reply.body.payload = Payload::GenerateOk {guid};
                serde_json::to_writer(&mut *output, &reply).context("serialize response to generate")?;
                output.write_all(b"\n").context("write trailing newline")?;
            },
            Payload::GenerateOk { .. } => {}  //bail!("received generate_ok message"),
        }

        Ok(())
    }
}


fn main() -> Result<()> {
    main_loop::<_, UniqueNode, _, _>(())
}
