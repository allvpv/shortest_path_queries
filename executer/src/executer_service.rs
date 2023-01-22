use std::sync::atomic::{AtomicU32, Ordering};
use tonic::{Request, Response, Result};

use generated::executer::executer_server::Executer;
use generated::executer::{QueryData, QueryFinished, HealthCheckResponse, HealthCheckRequest};
use generated::manager::manager_service_client::ManagerServiceClient;
use tonic::transport::Channel;
use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use futures::TryFutureExt;

use crate::query_coordinator::QueryCoordinator;
use crate::workers_connection::Worker;
use crate::workers_connection::get_sorted_workers_addresses;

pub type NodeId = u64;
pub type ShortestPathLen = u64;

pub struct ExecuterService<'a> {
    workers: Vec<Worker>,
    query_id_counter: AtomicU32,
    manager: &'a mut ManagerServiceClient<Channel>
}

impl ExecuterService<'a> {
    pub fn new(
        workers: Vec<Worker>,
        manager: &mut ManagerServiceClient<Channel>
    ) -> Self {
        ExecuterService {
            workers,
            query_id_counter: AtomicU32::new(0),
            manager
        }
    }

    fn get_new_query_id(&self) -> u32 {
        self.query_id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[tonic::async_trait]
impl Executer<'a> for ExecuterService<'a> {
    async fn shortest_path_query(
        &self,
        request: Request<QueryData>,
    ) -> Result<Response<QueryFinished>> {
        let QueryData {
            node_id_from,
            node_id_to,
        } = request.into_inner();

        let query_id = self.get_new_query_id();
        debug!("`query_id` is: {query_id}");

        if node_id_from == node_id_to {
            Ok(Response::new(QueryFinished {
                shortest_path_len: Some(0),
            }))
        } else {
            let coordinator =
                QueryCoordinator::new(&self.workers, node_id_from, node_id_to, query_id).await?;
            let response = coordinator.shortest_path_query().await?;

            Ok(Response::new(response))
        }
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>> {
        // get addresses to check that partitioner is alive
        let addresses = get_sorted_workers_addresses(self.manager).await?;

        let address_vec: Vec<String> = addresses.iter().map(|a| a.address.to_string()).collect();
        let worker_vec: Vec<String> = self.workers.iter().map(|w| w.address.to_string()).collect();
        assert_eq!(address_vec, worker_vec);

        // for address in addresses send AreNodesPresent request, just to check it returns some ArePresent instead of error/timeout
        let message = generated::worker::NodeIds {
            node_from_id: 1337,
            node_to_id: 71830,
        };

        let mut futs = self.workers
            .iter_mut()
            .enumerate()
            .map(|(idx, worker)| {
                worker
                    .channel
                    .are_nodes_present(Request::new(message.clone()))
                    .map_ok(move |resp| (idx, resp))
            })
            .collect::<FuturesUnordered<_>>();

        while let Some((_worker_idx, response)) = futs.try_next().await? {
            let _result = response.into_inner();
        }

        Ok(Response::new(HealthCheckResponse {
            status: 1,
        }))
    }
}
