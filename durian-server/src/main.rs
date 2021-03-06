#[macro_use]
extern crate capnp_rpc;

#[macro_use]
extern crate log;

pub mod durian_capnp {
    include!(concat!(env!("OUT_DIR"), "/durian_capnp.rs"));
}
mod executor_impl;
mod provider_adaptor;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use durian_capnp::executor;
use executor_impl::ExecutorImpl;
use futures::{AsyncReadExt, FutureExt, TryFutureExt};
use std::net::ToSocketAddrs;
use tokio::net::TcpListener;
use log::Level;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 2 {
        println!("usage: {} HOST:PORT", args[0]);
        return Ok(());
    }

    let addr = args[1]
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let mut listener = TcpListener::bind(&addr).await?;
            let executor_impl = ExecutorImpl::new();
            let executor: executor::Client = capnp_rpc::new_client(executor_impl);

            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system = RpcSystem::new(Box::new(network), Some(executor.clone().client));
                tokio::task::spawn_local(Box::pin(
                    rpc_system
                        .map_err(|e| println!("error: {:?}", e))
                        .map(|_| ()),
                ));
            }
        })
        .await
}
