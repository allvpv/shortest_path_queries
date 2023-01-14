use std::sync::atomic::{AtomicU32, Ordering};
use tonic::{Request, Response, Result};

use generated::executer::executer_server::Executer;
use generated::executer::{QueryData, QueryFinished};

use crate::query_coordinator::QueryCoordinator;
use crate::workers_connection::Worker;

pub type NodeId = u64;
pub type ShortestPathLen = u64;

pub struct ExecuterService {
    workers: Vec<Worker>,
    query_id_counter: AtomicU32,
}

impl ExecuterService {
    pub fn new(workers: Vec<Worker>) -> Self {
        ExecuterService {
            workers,
            query_id_counter: AtomicU32::new(0),
        }
    }

    fn get_new_query_id(&self) -> u32 {
        self.query_id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[tonic::async_trait]
impl Executer for ExecuterService {
    async fn shortest_path_query(
        &self,
        request: Request<QueryData>,
    ) -> Result<Response<QueryFinished>> {
        let QueryData {
            node_id_from,
            node_id_to,
        } = request.into_inner();

        let query_id = self.get_new_query_id();
        let coordinator =
            QueryCoordinator::new(&self.workers, node_id_from, node_id_to, query_id).await?;
        println!("Query coordinator created, id: {}", query_id);
        let response = coordinator.shortest_path_query().await?;

        Ok(Response::new(response))
    }
}
