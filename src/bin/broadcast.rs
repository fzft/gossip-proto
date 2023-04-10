use std::collections::{HashMap, HashSet};
use std::io::{StdoutLock, Write};
use std::ptr::null_mut;
use std::sync::mpsc::Sender;
use std::time::Duration;

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
    TopologyOk,
    Gossip { seen: HashSet<usize> },
}

enum InjectedPayload {
    Gossip
}

struct BroadcastNode {
    node: String,
    id: usize,
    message: HashSet<usize>,
    known: HashMap<String, HashSet<usize>>,
    neighborhood: Vec<String>,
    msg_communicated: HashMap<usize, HashSet<usize>>,
}

impl Node<(), Payload, InjectedPayload> for BroadcastNode {
    fn from_init(_state: (), init: Init, tx: Sender<Event<Payload, InjectedPayload>>) -> Result<Self> where Self: Sized {
        std::thread::spawn(move || {
            // generate gossip event
            loop {
                std::thread::sleep(Duration::from_millis(300));
                if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                    break;
                }
            }
        });

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
        input: Event<Payload, InjectedPayload>,
        output: &mut StdoutLock)
        -> Result<()> {
        match input {
            Event::Message(input) => {
                let mut reply = input.into_reply(Some(&mut self.id));
                match reply.body.payload {
                    Payload::Gossip { seen } => {
                        self.known.get_mut(&reply.dst).expect("got gossip from unknown node").extend(seen.iter().copied());
                        self.message.extend(seen);
                    }

                    Payload::Broadcast { message } => {
                        reply.body.payload = Payload::BroadcastOk;
                        reply.send(output).context("reply to broadcast")?;
                        self.message.insert(message);
                    }
                    Payload::Read { .. } => {
                        reply.body.payload = Payload::ReadOk {
                            messages: self.message.clone()
                        };
                        reply.send(output).context("reply to read")?;
                    }

                    Payload::Topology { mut topology } => {
                        self.neighborhood = topology.remove(&self.node).unwrap_or_else(|| panic!("no topology given for node {}", self.node));
                        reply.body.payload = Payload::TopologyOk;
                        reply.send(output).context("reply to topology")?;
                    }
                    _ => {}
                }
            }
            Event::Injected(payload) => {
                match payload {
                    InjectedPayload::Gossip => {
                        for n in &self.neighborhood {
                            let n_knows = &self.known[n];
                            Message {
                                src: self.node.clone(),
                                dst: n.clone(),
                                body: Body {
                                    id: None,
                                    payload: Payload::Gossip {
                                        seen: self.message.iter().copied().filter(|n| {
                                            !n_knows.contains(n)
                                        }).collect()
                                    },
                                    in_reply_to: None,
                                },
                            }
                                .send(output).with_context(|| format!("gossip to {}", n))?;
                        }
                    }
                }
            }
            Event::EOF => {}
        };

        Ok(())
    }
}


fn main() -> Result<()> {
    main_loop::<_, BroadcastNode, _, _>(())
}
