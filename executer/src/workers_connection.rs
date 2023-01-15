use futures::Future;
use log::Level::Debug;
use tonic::transport::{Channel, Error};
use tonic::Status;

use generated::manager::manager_service_client::ManagerServiceClient;
use generated::worker::worker_client::WorkerClient;

pub type WorkerId = u32;

#[derive(Clone)]
pub struct Worker {
    pub id: WorkerId,
    pub channel: WorkerClient<Channel>,
}

pub type WorkerAddrList = Vec<generated::manager::workers_list::WorkerEntry>;
pub type WorkerList = Vec<Worker>;

// Get workers addresses sorted by ID
pub async fn get_sorted_workers_addresses(
    manager: &mut ManagerServiceClient<Channel>,
) -> Result<WorkerAddrList, Status> {
    use tonic::Request;

    info!("getting workers list");

    let mut workers = manager
        .get_workers_list(Request::new(()))
        .await?
        .into_inner()
        .workers;

    workers.sort_by_key(|w| w.worker_id);

    if log_enabled!(Debug) {
        debug!("got {} addresses", workers.len());

        for worker in workers.iter() {
            debug!(" -> '{}'", worker.address);
        }
    }

    Ok(workers)
}

// Return connection to workers; returned list keeps the order of the input list
pub fn connect_to_all_workers(
    addrs: WorkerAddrList,
) -> impl Future<Output = Result<WorkerList, Error>> {
    info!("connecting to workers");

    use futures::TryFutureExt;

    let workers_connect = addrs.into_iter().map(|w| {
        debug!(
            " -> connecting to worker[id {}] at address '{}'",
            w.worker_id, w.address
        );

        WorkerClient::connect(w.address).map_ok(move |channel| Worker {
            id: w.worker_id,
            channel,
        })
    });

    futures::future::try_join_all(workers_connect)
}
