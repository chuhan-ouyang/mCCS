#![feature(peer_credentials_unix_socket)]
#![feature(drain_filter)]
#![feature(strict_provenance)]

pub mod config;
pub mod control;
pub mod daemon;
pub mod cuda;
pub mod transport;
pub mod communicator;
pub mod proxy;
pub mod resources;