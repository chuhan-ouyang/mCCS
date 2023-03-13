pub mod engine;
pub mod op;
pub mod command;
pub mod init;
pub mod message;
pub mod plan;
pub mod task;

use crate::comm::HostIdent;

pub struct DeviceInfo {
    pub host: HostIdent,
    pub cuda_device_idx: i32,
}