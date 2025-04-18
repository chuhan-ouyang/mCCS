#![feature(unix_socket_ancillary_data)]
#![feature(peer_credentials_unix_socket)]
#![feature(slice_index_methods)]
use std::io;
use std::os::unix::net::UCred;

use thiserror::Error;

/// Re-exports ipc_channel
pub mod ipc_channel;
/// Re-exports shmem_ipc
pub mod shmem_ipc;
pub(crate) use crate::shmem_ipc::{ShmReceiver, ShmSender};

/// Common data structures passed between client and server
pub mod control;

/// Provides Range
pub mod buf;

/// Provides DomainSocket
pub mod unix;

/// Provides ShmObject
pub(crate) mod shmobj;
pub(crate) use shmobj::ShmObject;

/// Provides Customer and Service
pub mod customer;
pub mod service;

pub mod channel;

#[derive(Debug, Error)]
pub enum TryRecvError {
    #[error("Empty")]
    Empty,
    #[error("Disconnected")]
    Disconnected,
    #[error("Other: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Debug, Error)]
pub enum IpcRecvError {
    #[error("Disconnected")]
    Disconnected,
    #[error("Other: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Debug, Error)]
pub enum IpcSendError {
    #[error("Bincode: {0}")]
    Bincode(bincode::Error),
    #[error("Crossbeam")]
    Crossbeam,
    #[error("Other: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Debug, Error)]
pub enum RecvFdError {
    #[error("Empty")]
    Empty,
    #[error("Disconnected")]
    Disconnected,
    #[error("Other: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO Error {0}")]
    Io(#[from] io::Error),
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("IPC send error: {0}")]
    IpcSend(IpcSendError),
    #[error("IPC recv error")]
    IpcRecv(IpcRecvError),
    #[error("IPC try recv error")]
    TryRecv(TryRecvError),
    #[error("DomainSocket error: {0}")]
    UnixDomainSocket(#[from] unix::Error),
    #[error("Send fd error: {0}")]
    SendFd(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Recv fd error: {0}")]
    RecvFd(RecvFdError),
    #[error("Try recv fd error: {0}")]
    TryRecvFd(TryRecvError),
    #[error("Shared memory queue error: {0}")]
    ShmIpc(#[from] shmem_ipc::ShmIpcError),
    #[error("Shared memory queue ringbuf error: {0}")]
    ShmRingbuf(#[from] shmem_ipc::ShmRingbufError),
    #[error("ShmObject error: {0}")]
    ShmObj(#[from] shmobj::Error),
    #[error("Expect a credential from the peer")]
    EmptyCredential,
    #[error("Credential mismatch {0:?} vs {1:?}")]
    CredentialMismatch(UCred, UCred),
    #[error("Control plane error {0}: {1}")]
    ControlPlane(&'static str, control::Error),
}

impl From<crate::ipc_channel::TryRecvError> for TryRecvError {
    fn from(other: crate::ipc_channel::TryRecvError) -> Self {
        use crate::ipc_channel::IpcRecvError as IRE;
        use crate::ipc_channel::TryRecvError as ITRE;
        match other {
            ITRE::Empty => TryRecvError::Empty,
            ITRE::IpcError(e) => match e {
                IRE::Disconnected => TryRecvError::Disconnected,
                IRE::Io(err) => TryRecvError::Other(Box::new(err)),
                IRE::Bincode(err) => TryRecvError::Other(Box::new(err)),
            },
        }
    }
}

impl From<crate::ipc_channel::IpcSendError> for IpcSendError {
    fn from(other: crate::ipc_channel::IpcSendError) -> Self {
        IpcSendError::Bincode(other)
    }
}

impl From<crate::ipc_channel::IpcRecvError> for IpcRecvError {
    fn from(other: crate::ipc_channel::IpcRecvError) -> Self {
        use crate::ipc_channel::IpcRecvError as IRE;
        match other {
            IRE::Disconnected => IpcRecvError::Disconnected,
            IRE::Io(err) => IpcRecvError::Other(Box::new(err)),
            IRE::Bincode(err) => IpcRecvError::Other(Box::new(err)),
        }
    }
}

pub(crate) const MAX_MSG_LEN: usize = 65536;
