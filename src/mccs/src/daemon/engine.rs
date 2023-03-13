use std::os::raw::c_void;
use std::collections::HashMap;

use crossbeam::channel::{Sender, Receiver};

use cuda_runtime_sys::{cudaMalloc, cudaIpcGetMemHandle, cudaSetDevice};
use cuda_runtime_sys::cudaError;
use cuda_runtime_sys::cudaIpcMemHandle_t;

use ipc::customer::ShmCustomer;
use ipc::mccs::command;
use ipc::mccs::dp;
use ipc::mccs::handle::{CudaMemHandle, CommunicatorHandle};

use crate::comm::CommunicatorId;
use crate::proxy::command::{ProxyCommand, ProxyCompletion, InitCommunicator, AllGather};

use super::{Error, DaemonId};

pub type CustomerType =
    ShmCustomer<command::Command, command::Completion, dp::WorkRequestSlot, dp::CompletionSlot>;

pub struct CommunicatorDelegation {
    pub comm_id: u32,
    pub cuda_device_idx: usize,
}

pub struct DaemonEngine {
    pub(crate) id: DaemonId,
    pub(crate) customer: CustomerType,
    pub(crate) proxy_command_tx: Vec<Sender<ProxyCommand>>,
    pub(crate) proxy_completion_rx: Vec<Receiver<ProxyCompletion>>,
    pub(crate) device_mem: HashMap<u64, usize>,
    pub(crate) comm_delegation: HashMap<CommunicatorHandle, CommunicatorDelegation>,
}

impl DaemonEngine {
    pub fn mainloop(&mut self) {
        loop {
            self.check_cmd().unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Progress(usize),
    Disconnected,
}

use Status::Progress;

impl DaemonEngine {
    fn process_cmd(
        &mut self,
        req: &command::Command,
    ) -> Result<Option<command::CompletionKind>, Error> {
        use ipc::mccs::command::{Command, CompletionKind};
        match req {
            Command::CudaMalloc(dev_idx, size) => {
                unsafe { 
                    let error = cudaSetDevice(*dev_idx as _);
                    if error != cudaError::cudaSuccess {
                        panic!("cudaSetDevice");
                }
                }
                let mut dev_ptr: *mut c_void = std::ptr::null_mut();
                let err = unsafe { cudaMalloc(&mut dev_ptr as *mut _, *size) };
                if err != cudaError::cudaSuccess {
                    panic!("cudaMalloc failed")
                }
                let mut handle = cudaIpcMemHandle_t::default();
                let err = unsafe { cudaIpcGetMemHandle(&mut handle as *mut _, dev_ptr) };
                if err != cudaError::cudaSuccess {
                    panic!("cudaIpcGetMemHandle failed")
                }
                self.device_mem.insert(0, dev_ptr as usize);
                let return_handle = CudaMemHandle(handle.reserved);
                Ok(Some(CompletionKind::CudaMalloc(return_handle)))
            },
            Command::InitCommunicator(init) => {
                let proxy_init = InitCommunicator {
                    communicator_id: CommunicatorId(init.id),
                    rank: init.rank,
                    num_ranks: init.num_ranks,
                };
                let proxy_cmd = ProxyCommand::InitCommunicator(proxy_init);
                self.proxy_command_tx[init.cuda_device_idx].send(proxy_cmd).unwrap();
                let res = self.proxy_completion_rx[init.cuda_device_idx].recv().unwrap();
                match res {
                    ProxyCompletion::InitCommunicator => {},
                    _ => panic!("unexpected result"),
                };
                let comm_handle = CommunicatorHandle((init.id as u64) << 32 + init.rank);
                let comm = CommunicatorDelegation {
                    comm_id: init.id,
                    cuda_device_idx: init.cuda_device_idx,
                };
                self.comm_delegation.insert(comm_handle, comm);
                
                Ok(Some(CompletionKind::InitCommunicator(comm_handle)))
            },
            Command::AllGather(all_gather) => {
                let comm = self.comm_delegation.get(&all_gather.comm).unwrap();
                let proxy_all_gather = AllGather {
                    communicator_id: CommunicatorId(comm.comm_id),
                    send_buf_addr: *self.device_mem.get(&0).unwrap(),
                    recv_buf_addr: *self.device_mem.get(&0).unwrap(),
                    size: all_gather.size,
                };
                let proxy_cmd = ProxyCommand::AllGather(proxy_all_gather);
                self.proxy_command_tx[comm.cuda_device_idx].send(proxy_cmd).unwrap();
                let res = self.proxy_completion_rx[comm.cuda_device_idx].recv().unwrap();
                match res {
                    ProxyCompletion::AllGather => {},
                    _ => panic!("unexpected result"),
                }
                Ok(Some(CompletionKind::AllGather))
            },
        }
    }

    fn check_cmd(&mut self) -> Result<Status, Error> {
        match self.customer.try_recv_cmd() {
            Ok(req) => {
                let result = self.process_cmd(&req);
                match result {
                    Ok(Some(res)) => self.customer.send_comp(command::Completion(Ok(res)))?,
                    Ok(None) => return Ok(Progress(0)),
                    Err(_e) => panic!(),
                }
                Ok(Progress(1))
            }
            Err(ipc::TryRecvError::Empty) => {
                Ok(Progress(0))
            }
            Err(ipc::TryRecvError::Disconnected) => Ok(Status::Disconnected),
            Err(ipc::TryRecvError::Other(_e)) => Err(Error::IpcTryRecv),
        }
    }
}