mod executer_service;
mod query_coordinator;
mod workers_connection;

pub mod manager {
    tonic::include_proto!("manager");
}

pub mod worker {
    tonic::include_proto!("worker");
}

pub mod executer {
    tonic::include_proto!("executer");
}

use clap::Parser;
use tonic::transport::Server;

use executer::executer_server::ExecuterServer;
use executer_service::ExecuterService;
use manager::manager_service_client::ManagerServiceClient;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    manager_addr: String,
    #[arg(long)]
    listening_addr: String,
}

pub struct ErrorCollection {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Connecting to manager");
    let mut manager = ManagerServiceClient::connect(args.manager_addr).await?;

    println!("Getting workers list");
    let addrs = workers_connection::get_sorted_workers_addresses(&mut manager).await?;

    println!("Connecting to workers");
    let workers = workers_connection::connect_to_all_workers(addrs).await?;

    println!("Running the server");
    let listening_addr = match args.listening_addr.parse() {
        Ok(addr) => addr,
        Err(err) => {
            return Err(format!(
                "Cannot parse listening address `{}`: {err}",
                args.listening_addr
            )
            .into())
        }
    };

    let service = ExecuterService::new(workers);
    let server = ExecuterServer::new(service);

    Server::builder()
        .add_service(server)
        .serve(listening_addr)
        .await?;

    Ok(())
}