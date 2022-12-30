use tonic::{Request, Response, Result};

use crate::executer::executer_server::Executer;
use crate::executer::{QueryData, QueryFinished};
use crate::query_coordinator::QueryCoordinator;
use crate::workers_connection::Worker;

pub type NodeId = u64;
pub type ShortestPathLen = u64;

pub struct ExecuterService {
    workers: Vec<Worker>,
}

impl ExecuterService {
    pub fn new(workers: Vec<Worker>) -> Self {
        ExecuterService { workers }
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

        let coordinator = QueryCoordinator::new(&self.workers, node_id_from, node_id_to).await?;
        let response = coordinator.shortest_path_query().await?;

        Ok(Response::new(response))
    }
}
