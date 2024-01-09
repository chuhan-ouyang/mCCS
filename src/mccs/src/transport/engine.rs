use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};

use crossbeam::channel::TryRecvError;
use futures::future::BoxFuture;
use futures::FutureExt;

use crate::registry::GlobalRegistry;
use crate::utils::duplex_chan::DuplexChannel;
use crate::utils::pool::WorkPool;

use super::channel::ConnType;
use super::message::{TransportEngineReply, TransportEngineRequest};
use super::op::TransportOp;
use super::queue::TransrportOpQueue;
use super::transporter::{AgentMessage, AnyResources, TransportAgentId, Transporter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TransportEngineId {
    pub cuda_device_idx: i32,
    pub index: u32,
}

pub struct TransportAgent {
    transporter: &'static dyn Transporter,
    agent_resources: AnyResources,
}

enum AsyncTaskResult {
    Setup {
        setup_resources: AnyResources,
        reply: AgentMessage,
    },
    Connect {
        agent_resources: AnyResources,
        reply: AgentMessage,
    },
}

pub struct AsyncTask {
    agent_id: TransportAgentId,
    transporter: &'static dyn Transporter,
    task: BoxFuture<'static, AsyncTaskResult>,
}

fn new_setup_task(
    transporter: &'static dyn Transporter,
    id: TransportAgentId,
    request: AgentMessage,
    registry: &GlobalRegistry,
) -> AsyncTask {
    let setup = match id.peer_conn.conn_type {
        ConnType::Send => {
            transporter.agent_send_setup(id, request, Arc::clone(&registry.transport_catalog))
        }
        ConnType::Recv => {
            transporter.agent_recv_setup(id, request, Arc::clone(&registry.transport_catalog))
        }
    };
    let task = setup.map(|res| {
        let (resources, reply) = res.unwrap();
        AsyncTaskResult::Setup {
            setup_resources: resources,
            reply,
        }
    });
    let pinned = Box::pin(task);
    AsyncTask {
        agent_id: id,
        transporter,
        task: pinned,
    }
}

fn new_connect_task(
    transporter: &'static dyn Transporter,
    id: TransportAgentId,
    request: AgentMessage,
    setup_resources: Option<AnyResources>,
) -> AsyncTask {
    let connect = match id.peer_conn.conn_type {
        ConnType::Send => transporter.agent_send_connect(id, request, setup_resources),
        ConnType::Recv => transporter.agent_recv_connect(id, request, setup_resources),
    };
    let task = connect.map(|res| {
        let (resources, reply) = res.unwrap();
        AsyncTaskResult::Connect {
            agent_resources: resources,
            reply,
        }
    });
    let pinned = Box::pin(task);
    AsyncTask {
        agent_id: id,
        transporter,
        task: pinned,
    }
}

pub struct TransportEngineResources {
    pub agent_setup: HashMap<TransportAgentId, AnyResources>,
    pub agent_connected: HashMap<TransportAgentId, TransportAgent>,
    pub proxy_chan: Vec<DuplexChannel<TransportEngineReply, TransportEngineRequest>>,
    pub global_registry: Arc<GlobalRegistry>,
}

impl TransportEngineResources {
    fn progress_op(&mut self, _op: &mut TransportOp) -> bool {
        // TODO
        true
    }

    fn progress_async_task(&mut self, task: &mut AsyncTask) -> bool {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);
        let poll = task.task.as_mut().poll(&mut cx);
        match poll {
            Poll::Ready(result) => {
                match result {
                    AsyncTaskResult::Setup {
                        setup_resources,
                        reply,
                    } => {
                        let reply = TransportEngineReply::AgentSetup(task.agent_id, reply);
                        self.proxy_chan[task.agent_id.client_cuda_dev as usize]
                            .tx
                            .send(reply)
                            .unwrap();
                        self.agent_setup.insert(task.agent_id, setup_resources);
                    }
                    AsyncTaskResult::Connect {
                        agent_resources,
                        reply,
                    } => {
                        let connected = TransportAgent {
                            transporter: task.transporter,
                            agent_resources,
                        };
                        let reply = TransportEngineReply::AgentConnect(task.agent_id, reply);
                        self.proxy_chan[task.agent_id.client_cuda_dev as usize]
                            .tx
                            .send(reply)
                            .unwrap();
                        self.agent_connected.insert(task.agent_id, connected);
                    }
                }
                true
            }
            Poll::Pending => false,
        }
    }
}

pub struct TransportEngine {
    pub id: TransportEngineId,
    pub resources: TransportEngineResources,
    pub async_tasks: WorkPool<AsyncTask>,
    pub op_queue: TransrportOpQueue,
}

impl TransportEngine {
    fn progress_ops(&mut self) {
        self.op_queue
            .progress_ops(|op| self.resources.progress_op(op));
    }

    fn progress_async_tasks(&mut self) {
        self.async_tasks
            .progress(|x| self.resources.progress_async_task(x));
    }

    fn check_proxy_requests(&mut self) {
        for rx in self.resources.proxy_chan.iter_mut().map(|c| &mut c.rx) {
            match rx.try_recv() {
                Ok(request) => {
                    let task = match request {
                        TransportEngineRequest::AgentSetup(transporter, agent_id, request) => {
                            new_setup_task(
                                transporter,
                                agent_id,
                                request,
                                &self.resources.global_registry,
                            )
                        }
                        TransportEngineRequest::AgentConnect(transporter, agent_id, request) => {
                            let setup_resources = self.resources.agent_setup.remove(&agent_id);
                            new_connect_task(transporter, agent_id, request, setup_resources)
                        }
                        TransportEngineRequest::AgentTransportOp(agend_id, tx_op) => {
                            todo!()
                        }
                    };
                    self.async_tasks.enqueue(task);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    unreachable!("Proxy engines shall never shutdown")
                }
            }
        }
    }
}

impl TransportEngine {
    pub fn mainloop(&mut self) {
        loop {
            self.check_proxy_requests();
            self.progress_async_tasks();
            self.progress_ops();
        }
    }
}
