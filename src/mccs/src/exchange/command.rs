use std::net::SocketAddr;

use crate::bootstrap::BootstrapHandle;
use crate::comm::CommunicatorId;

pub enum ExchangeCommand {
    RegisterBootstrapHandle(CommunicatorId, BootstrapHandle),
    // communicator id, root mccs exchange engine listen addr
    RecvBootstrapHandle(CommunicatorId, SocketAddr),
    RemoveCommunicator(CommunicatorId),
}

pub enum ExchangeNotification {
    RegisterBootstrapHandle,
    RecvBootstrapHandle(CommunicatorId, BootstrapHandle),
}
