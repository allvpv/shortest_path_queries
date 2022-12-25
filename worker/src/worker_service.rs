pub mod worker {
    tonic::include_proto!("worker");
}

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use worker::worker_server::Worker;
use worker::{IsPresent, NodeId as NodeIdProto, RequestDjikstra, ResponseDjikstra};

use crate::graph_store::{IdIdxMapping, SPQGraph};
use crate::query_processor::{QueryId, QueryProcessor};
use crate::ErrorCollection;

use crate::worker_service::worker::request_djikstra::QueryData;

#[derive(Debug)]
enum QueryProcessorHolder {
    Busy, // The query is pending, QueryProcessor was moved to blocking thread
    Ready(QueryProcessor),
}

type QueryIdProcessorMap = HashMap<QueryId, QueryProcessorHolder>;

#[derive(Debug)]
pub struct WorkerService {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    queries: Mutex<QueryIdProcessorMap>,
}

impl WorkerService {
    pub fn new(graph: SPQGraph, mapping: IdIdxMapping) -> Self {
        WorkerService {
            graph: Arc::new(graph),
            mapping: Arc::new(mapping),
            queries: Mutex::new(QueryIdProcessorMap::new()),
        }
    }

    fn get_query_processor(&self, query_id: QueryId) -> Result<Option<QueryProcessor>, Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let mut queries = self
            .queries
            .lock()
            .map_err(ErrorCollection::locking_mutex)?;

        match queries.entry(query_id) {
            Vacant(entry) => {
                // No QueryProcessor for this query_id was created, but we will create one
                // soon. Insert Busy into holder and return None.
                entry.insert(QueryProcessorHolder::Busy);
                Ok(None)
            }
            Occupied(mut entry) => {
                // QueryProcessor for this query_id was already created.
                match entry.insert(QueryProcessorHolder::Busy) {
                    // If it is busy, than there is error in Executer; new request for this
                    // query came, before previous was finished.
                    QueryProcessorHolder::Busy => Err(ErrorCollection::duplicated_request()),
                    QueryProcessorHolder::Ready(processor) => Ok(Some(processor)),
                }
            }
        }
    }

    fn get_or_create_query_processor(&self, data: &QueryData) -> Result<QueryProcessor, Status> {
        let processor = match self.get_query_processor(data.query_id)? {
            None => {
                let graph = Arc::clone(&self.graph);
                let mapping = Arc::clone(&self.mapping);
                QueryProcessor::new(graph, mapping, data.final_node_id)
            }
            Some(processor) => processor,
        };

        Ok(processor)
    }

    fn put_query_processor(
        &self,
        query_id: QueryId,
        query_processor: QueryProcessor,
    ) -> Result<(), Status> {
        let mut queries = self
            .queries
            .lock()
            .map_err(ErrorCollection::locking_mutex)?;

        let holder = queries.get_mut(&query_id).unwrap();
        debug_assert!(matches!(holder, QueryProcessorHolder::Busy));
        *holder = QueryProcessorHolder::Ready(query_processor);

        Ok(())
    }
}

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn is_node_present(
        &self,
        request: Request<NodeIdProto>,
    ) -> Result<Response<IsPresent>, Status> {
        let node_id = request.get_ref().node_id;
        let present = self.mapping.contains_key(&node_id);

        Ok(Response::new(IsPresent { present }))
    }

    type UpdateDjikstraStream =
        Pin<Box<dyn Stream<Item = Result<ResponseDjikstra, Status>> + Send + 'static>>;

    async fn update_djikstra(
        &self,
        request: Request<tonic::Streaming<RequestDjikstra>>,
    ) -> Result<Response<Self::UpdateDjikstraStream>, Status> {
        let mut inbound = request.into_inner();
        let next_message = inbound.message().await?.and_then(|r| r.message_type);

        use crate::worker_service::worker::request_djikstra::MessageType::QueryData;

        let query_data = {
            if let Some(QueryData(data)) = next_message {
                data
            } else {
                return Err(ErrorCollection::wrong_first_message());
            }
        };

        let mut query_processor = self.get_or_create_query_processor(&query_data)?;
        query_processor
            .apply_update(&mut inbound, query_data.smallest_foreign_element)
            .await?;

        // Move the processor in and out the task to satisfy borrow checker
        let (query_processor, result_vec) =
            tokio::task::spawn_blocking(move || query_processor.djikstra_step())
                .await
                .expect("QueryProcessor djikstra_step task panicked")?;

        self.put_query_processor(query_data.query_id, query_processor)?;
        let result_iter = result_vec.into_iter().map(Ok);
        let output = futures::stream::iter(result_iter);

        Ok(Response::new(Box::pin(output)))
    }
}

impl ErrorCollection {
    fn wrong_first_message() -> Status {
        Status::invalid_argument("First message in UpdateDjikstra stream must be query_id")
    }

    fn locking_mutex(e: impl std::error::Error) -> Status {
        Status::internal(format!("Internal error while locking mutex: {e}"))
    }

    fn duplicated_request() -> Status {
        Status::invalid_argument(
            "Error. Executer requested UpdateDjikstra, while another \
             UpdateDjikstra on this query was already pending",
        )
    }
}
