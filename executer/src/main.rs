extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod executer_service;
mod query_coordinator;
mod workers_connection;

use std::env;
use std::net::ToSocketAddrs;

use local_ip_address::local_ip;
use tonic::transport::Server;

use generated::executer::executer_server::ExecuterServer;
use generated::manager::manager_service_client::ManagerServiceClient;

use crate::executer_service::ExecuterService;

pub struct ErrorCollection {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    info!("connecting to manager");
    let manager_addr = env::var("PARTITIONER_IP").unwrap();
    let mut manager = ManagerServiceClient::connect(manager_addr).await?;
    info!("connected to manager");

    let addresses = workers_connection::get_sorted_workers_addresses(&mut manager).await?;
    let workers = workers_connection::connect_to_all_workers(addresses).await?;

    info!("creating the server");
    let service = ExecuterService::new(workers);
    let server = ExecuterServer::new(service);

    let my_local_ip = local_ip()?;

    debug!("this is my local IP address: {:?}", my_local_ip);

    let listening_addr = format!("{}:{}", my_local_ip, 49999)
        .to_socket_addrs()
        .map_err(|e| format!("failed to parse own address: {e:?}"))?
        .next()
        .ok_or_else(|| "no own address found".to_string())?;

    info!("starting server at address: '{}'", listening_addr);
    Server::builder()
        .add_service(server)
        .serve(listening_addr)
        .await?;

    Ok(())
}
