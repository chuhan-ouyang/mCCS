use crossbeam::channel::Sender;
use dashmap::DashMap;

use super::engine::TransportEngineId;
use super::transporter::TransportAgentId;
use crate::message::ControlRequest;

const MAX_CONNS_PER_ENGINE: usize = 8;
const MAX_NUM_ENGINES_PER_DEVICE: usize = 8;

pub struct TransportDelegator {
    active_connections: DashMap<i32, Vec<(u32, usize)>>,
    agent_assignments: DashMap<TransportAgentId, TransportEngineId>,
    shutdown_engines: DashMap<i32, Vec<u32>>,
}

impl TransportDelegator {
    pub fn new() -> Self {
        TransportDelegator {
            active_connections: DashMap::new(),
            agent_assignments: DashMap::new(),
            shutdown_engines: DashMap::new(),
        }
    }
}

impl TransportDelegator {
    pub fn assign_transport_engine(
        &self,
        cuda_dev: i32,
        agent: TransportAgentId,
        control: &mut Sender<ControlRequest>,
    ) -> TransportEngineId {
        let mut engines = self
            .active_connections
            .entry(cuda_dev)
            .or_insert_with(Vec::new);
        let num_engines = engines.len();

        let least_load = engines.iter_mut().min_by_key(|x| x.1);

        if let Some((engine_idx, conns)) = least_load {
            if (*conns < MAX_CONNS_PER_ENGINE) || (num_engines >= MAX_NUM_ENGINES_PER_DEVICE) {
                let engine = TransportEngineId {
                    cuda_device_idx: cuda_dev,
                    index: *engine_idx,
                };
                *conns += 1;
                self.agent_assignments.insert(agent, engine);
                return engine;
            }
        }
        let reusable_indices = self.shutdown_engines.get_mut(&cuda_dev);
        let idx = if let Some(mut indices) = reusable_indices {
            indices.pop()
        } else {
            None
        };
        let idx = idx.unwrap_or_else(|| engines.iter().max_by_key(|x| x.0).map_or(0, |x| x.0 + 1));
        let new_engine = TransportEngineId {
            cuda_device_idx: cuda_dev,
            index: idx,
        };
        control
            .send(ControlRequest::NewTransportEngine(new_engine))
            .unwrap();
        new_engine
    }
}
