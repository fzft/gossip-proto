use std::collections::{HashMap, HashSet};
use std::io::{StdoutLock, Write};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use naruto::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Broadcast {
        message: usize
    },
    BroadcastOk,
    ReadOk {
      messages: HashSet<usize>
    },
    Read,
    Topology {
        topology: HashMap<String, Vec<String>>
    },
    TopologyOk
}

struct BroadcastNode {
    node: String,
    id: usize,
    message: HashSet<usize>,
    known: HashMap<String, HashSet<usize>>,
    neighborhood: Vec<String>,
    msg_communicated: HashMap<usize, HashSet<usize>>
}

impl Node<(), Payload> for BroadcastNode {
    fn from_init(_state: (), init: Init) -> Result<Self> where Self: Sized {
        Ok(Self {
            node: init.node_id,
            id: 1,
            message: HashSet::new(),
            known: init.node_ids.clone().into_iter().map(|nid| (nid, HashSet::new())).collect(),
            neighborhood: Vec::new(),
            msg_communicated: HashMap::new(),

        })
    }

    fn step(
        &mut self,
        input: Message<Payload>,
        output: &mut StdoutLock)
        -> Result<()> {
        let mut reply = input.into_reply(Some(&mut self.id));
        match reply.body.payload {
            Payload::Broadcast {message} => {
                reply.body.payload = Payload::BroadcastOk;
                serde_json::to_writer(&mut *output, &reply).context("serialize response to broadcast")?;
                output.write_all(b"\n").context("write trailing newline")?;
                self.message.insert(message);
            },
            Payload::Read { .. } => {
                reply.body.payload = Payload::ReadOk {
                    messages: self.message.clone()
                };
                serde_json::to_writer(&mut *output, &reply).context("serialize response to read")?;
                output.write_all(b"\n").context("write trailing newline")?;
            }  ,

            Payload::Topology {topology} => {
                self.neighborhood = topology.remove(&self.node).unwrap_or_else(||panic!("no topology given for node {}", self.node));
                reply.body.payload = Payload::TopologyOk;
                serde_json::to_writer(&mut *output, &reply).context("serialize response to topology")?;
                output.write_all(b"\n").context("write trailing newline")?;
            }
            _ => {}
        }

        Ok(())
    }
}


fn main() -> Result<()> {
    main_loop::<_, BroadcastNode, _>(())
}
