use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use futures::Stream;
use tonic::{Request, Response, Result, Status};

use generated::executer;
use generated::executer::executer_server::Executer;

use crate::query_coordinator::QueryCoordinator;
use crate::workers_connection::Worker;
use crate::ErrorCollection;

pub type NodeId = u64;
pub type ShortestPathLen = u64;
pub type QueryId = u32;

pub struct ExecuterService {
    workers: Vec<Worker>,
    query_id_counter: AtomicU32,
    query_coordinators: Mutex<HashMap<QueryId, Option<QueryCoordinator>>>,
}

impl ExecuterService {
    pub fn new(workers: Vec<Worker>) -> Self {
        ExecuterService {
            workers,
            query_id_counter: AtomicU32::new(0),
            query_coordinators: Mutex::new(HashMap::new()),
        }
    }

    fn get_new_query_id(&self) -> QueryId {
        self.query_id_counter.fetch_add(1, Ordering::Relaxed)
    }

    async fn send_forget_query(mut coordinator: QueryCoordinator) {
        match coordinator.send_forget_to_workers().await {
            Err(e) => warn!("Cannot send forget query to workers: {e:?}"),
            Ok(()) => (),
        }
    }

    fn get_query_coordinator(&self, query_id: QueryId) -> Result<QueryCoordinator, Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let mut coordinators = self.query_coordinators.lock().unwrap();
        let coordinator = match coordinators.entry(query_id) {
            Occupied(mut entry) => entry.insert(None),
            Vacant(_) => None,
        };

        coordinator.ok_or_else(|| ErrorCollection::query_invalid_or_busy(query_id))
    }
}

type NodeStream = Pin<Box<dyn Stream<Item = Result<executer::Node, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl Executer for ExecuterService {
    async fn shortest_path_query(
        &self,
        request: Request<executer::QueryData>,
    ) -> Result<Response<executer::QueryResults>> {
        let executer::QueryData {
            node_id_from,
            node_id_to,
        } = request.into_inner();

        let query_id = self.get_new_query_id();
        info!("`query_id` is: {query_id}");

        if node_id_from == node_id_to {
            Ok(Response::new(executer::QueryResults {
                shortest_path_len: Some(0),
                query_id: None,
            }))
        } else {
            let mut coordinator =
                QueryCoordinator::new(&self.workers, node_id_from, node_id_to, query_id).await?;
            let response = coordinator.shortest_path_query().await?;

            self.query_coordinators
                .lock()
                .unwrap()
                .insert(query_id, Some(coordinator));

            Ok(Response::new(response))
        }
    }

    type BacktrackPathForQueryStream = NodeStream;

    async fn backtrack_path_for_query(
        &self,
        request: Request<executer::QueryId>,
    ) -> Result<Response<NodeStream>> {
        let executer::QueryId { query_id } = request.into_inner();
        let coordinator = self.get_query_coordinator(query_id)?;

        unimplemented!();
    }

    async fn forget_query(&self, request: Request<executer::QueryId>) -> Result<Response<()>> {
        let executer::QueryId { query_id } = request.into_inner();
        let coordinator = self.get_query_coordinator(query_id)?;

        Self::send_forget_query(coordinator).await;
        self.query_coordinators.lock().unwrap().remove(&query_id);

        Ok(Response::new(()))
    }
}

impl ErrorCollection {
    fn query_invalid_or_busy(query_id: QueryId) -> Status {
        Status::invalid_argument(format!(
            "Query {query_id} does not exist or some request on this query is already pending"
        ))
    }
}
