mod graph_receiver;
mod graph_store;
mod request_processor;
mod worker_service;

use std::string::String;

use clap::Parser;
use tonic::transport::Server;

use crate::graph_receiver::GraphReceiver;
use crate::worker_service::worker::worker_server::WorkerServer;
use crate::worker_service::WorkerService;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    manager_addr: String,
    #[arg(long)]
    listening_addr: String,
}

pub struct ErrorCollection {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let num_cpus = num_cpus::get();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        // Our "blocking" threads will do CPU-bound tasks (as opposed to IO), so the upper limit
        // should be low (default max is 512).
        .max_blocking_threads(num_cpus - 1)
        .build()?
        .block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    use crate::graph_receiver::manager::manager_service_client::ManagerServiceClient;
    let client = ManagerServiceClient::connect(args.manager_addr).await?;
    let mut receiver = GraphReceiver::new(client).await?;

    println!("Worker id: {}", receiver.worker_id);

    receiver.receive_graph().await?;

    let service = WorkerService::new(receiver.graph, receiver.mapping);

    let listening_addr = match args.listening_addr.parse() {
        Ok(addr) => addr,
        Err(err) => {
            return Err(format!("Cannot parse address `{}`: {err}", args.listening_addr).into())
        }
    };

    let server = WorkerServer::new(service);

    Server::builder()
        .add_service(server)
        .serve(listening_addr)
        .await?;

    Ok(())
}
