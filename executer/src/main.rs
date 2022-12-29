mod workers_connection;
mod executer_service;

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

    Ok(())
}
