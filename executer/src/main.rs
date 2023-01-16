extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod executer_service;
mod query_coordinator;
mod workers_connection;

use clap::Parser;
use tonic::transport::Server;

use generated::executer::executer_server::ExecuterServer;
use generated::manager::manager_service_client::ManagerServiceClient;

use crate::executer_service::ExecuterService;

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
    pretty_env_logger::init();

    let listening_addr = match args.listening_addr.parse() {
        Ok(addr) => addr,
        Err(err) => {
            return Err(format!(
                "cannot parse listening address `{}`: {err}",
                args.listening_addr
            )
            .into())
        }
    };

    info!("connecting to manager");
    let mut manager = ManagerServiceClient::connect(args.manager_addr).await?;

    let addresses = workers_connection::get_sorted_workers_addresses(&mut manager).await?;
    let workers = workers_connection::connect_to_all_workers(addresses).await?;

    info!("creating the server");
    let service = ExecuterService::new(workers);
    let server = ExecuterServer::new(service);

    info!("starting server at address: '{}'", listening_addr);
    Server::builder()
        .add_service(server)
        .serve(listening_addr)
        .await?;

    Ok(())
}
