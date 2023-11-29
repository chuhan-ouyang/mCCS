pub mod device;

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

use crate::pattern::RingPattern;
use crate::proxy::plan::{ChanWorkSchedule, KernelPlan};
use crate::proxy::task::TaskQueue;
use crate::transport::channel::{ChannelId, CommChannel};
use crate::transport::NUM_PROTOCOLS;

use cuda_runtime_sys::{cudaEvent_t, cudaStream_t};
use device::CommDevResources;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommunicatorId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostIdent(pub SocketAddr);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerType {
    Local,
    IntraNode,
    InterNode,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub peer_type: PeerType,
    pub host: HostIdent,
    pub cuda_device_idx: i32,
}

pub struct Communicator {
    pub id: CommunicatorId,
    pub rank: usize,
    pub num_ranks: usize,
    pub peers_info: Vec<PeerInfo>,
    // channel_id -> CommChannel
    pub channels: HashMap<ChannelId, CommChannel>,
    pub profile: CommProfile,
    pub dev_resources: CommDevResources,

    pub task_queue: TaskQueue,

    pub plan_schedule: HashMap<ChannelId, ChanWorkSchedule>,
    pub unlaunched_plans: VecDeque<KernelPlan>,

    pub stream: cudaStream_t,
    pub event: cudaEvent_t,
}

// TBD
unsafe impl Send for Communicator {}

pub struct ChannelCommPattern {
    // channel id
    pub channel: ChannelId,
    pub ring: RingPattern,
}

pub struct CommProfile {
    pub buff_sizes: [usize; NUM_PROTOCOLS],
}
