use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use futures::Stream;
use tonic::{Result, Status};

use generated::executer;

use crate::query_coordinator::QueryCoordinator;
use crate::workers_connection::Worker;
use crate::ErrorCollection;

pub type NodeId = u64;
pub type ShortestPathLen = u64;
pub type QueryId = u32;

pub struct QueriesManager {
    workers: Vec<Worker>,
    query_id_counter: AtomicU32,
    query_coordinators: Mutex<HashMap<QueryId, Option<QueryCoordinator>>>,
}

impl QueriesManager {
    pub fn new(workers: Vec<Worker>) -> Self {
        QueriesManager {
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

    pub async fn shortest_path_query(
        &self,
        request: executer::QueryData,
    ) -> Result<executer::QueryResults> {
        let executer::QueryData {
            node_id_from,
            node_id_to,
        } = request;

        let query_id = self.get_new_query_id();
        info!("`query_id` is: {query_id}");

        let response = {
            if node_id_from == node_id_to {
                executer::QueryResults {
                    shortest_path_len: Some(0),
                    query_id: None,
                }
            } else {
                let mut coordinator =
                    QueryCoordinator::new(&self.workers, node_id_from, node_id_to, query_id)
                        .await?;
                let shortest_path_len = coordinator.shortest_path_query().await?;

                self.query_coordinators
                    .lock()
                    .unwrap()
                    .insert(query_id, Some(coordinator));

                executer::QueryResults {
                    shortest_path_len,
                    query_id: Some(query_id),
                }
            }
        };

        Ok(response)
    }

    pub fn get_backtrack_stream(
        &'static self,
        query_id: QueryId,
    ) -> impl Stream<Item = Result<executer::Node, Status>> + Send + 'static {
        async_stream::try_stream! {
            let mut coordinator = self.get_query_coordinator(query_id)?;

            let last_worker_idx = coordinator.last_worker_idx;
            let last_worker_id = coordinator.get_worker_id(last_worker_idx);

            let mut next_point = Some((last_worker_idx, last_worker_id, coordinator.node_id_to));

            yield executer::Node {
                node_id: coordinator.node_id_to,
                worker_id: last_worker_id,
            };

            while let Some((cur_worker_idx, cur_worker_id, current_node)) = next_point {
                let mut inbound = coordinator
                    .send_backtrack_request_to_worker(cur_worker_idx, current_node).await?;

                next_point = None;

                while let Some(node) = inbound.message().await? {

                    if let Some(worker_id) = node.worker_id {
                        let worker_idx = coordinator.find_worker_by_id(worker_id)?;
                        next_point = Some((worker_idx, worker_id, node.node_id));

                        yield executer::Node {
                            node_id: node.node_id,
                            worker_id: worker_id,
                        };
                    } else {
                        yield executer::Node {
                            node_id: node.node_id,
                            worker_id: cur_worker_id,
                        };
                    }
                }
            }

            self.query_coordinators
                .lock()
                .unwrap()
                .insert(query_id, Some(coordinator));
        }
    }

    pub async fn forget_query(&self, request: executer::QueryId) -> Result<(), Status> {
        let executer::QueryId { query_id } = request;
        let coordinator = self.get_query_coordinator(query_id)?;

        Self::send_forget_query(coordinator).await;
        self.query_coordinators.lock().unwrap().remove(&query_id);

        Ok(())
    }
}

impl ErrorCollection {
    fn query_invalid_or_busy(query_id: QueryId) -> Status {
        Status::invalid_argument(format!(
            "Query {query_id} does not exist or some request on this query is already pending"
        ))
    }
}
