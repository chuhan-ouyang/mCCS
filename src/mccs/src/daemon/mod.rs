use thiserror::Error;

pub mod engine;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("ipc-channel TryRecvError")]
    IpcTryRecv,
    #[error("Customer error: {0}")]
    Customer(#[from] ipc::Error),
}

pub type DaemonId = u64;
