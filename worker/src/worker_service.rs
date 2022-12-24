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
use crate::request_processor::{RequestId, RequestProcessor};
use crate::ErrorCollection;

#[derive(Debug)]
enum RequestProcessorHolder {
    Busy, // The request is pending, RequestProcessor was moved to blocking thread
    Ready(RequestProcessor),
}

type RequestIdProcessorMap = HashMap<RequestId, RequestProcessorHolder>;

#[derive(Debug)]
pub struct WorkerService {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    requests: Mutex<RequestIdProcessorMap>,
}

impl WorkerService {
    pub fn new(graph: SPQGraph, mapping: IdIdxMapping) -> Self {
        WorkerService {
            graph: Arc::new(graph),
            mapping: Arc::new(mapping),
            requests: Mutex::new(RequestIdProcessorMap::new()),
        }
    }

    fn get_request_processor(&self, request_id: RequestId) -> Result<RequestProcessor, Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let processor = {
            match self
                .requests
                .lock()
                .map_err(ErrorCollection::locking_mutex)?
                .entry(request_id)
            {
                Vacant(entry) => {
                    entry.insert(RequestProcessorHolder::Busy);
                    RequestProcessor::new(self.graph.clone(), self.mapping.clone())
                }
                Occupied(mut entry) => match entry.insert(RequestProcessorHolder::Busy) {
                    RequestProcessorHolder::Busy => {
                        return Err(ErrorCollection::duplicate_request())
                    }
                    RequestProcessorHolder::Ready(processor) => processor,
                },
            }
        };

        Ok(processor)
    }

    fn put_request_processor(
        &self,
        request_id: RequestId,
        request_processor: RequestProcessor,
    ) -> Result<(), Status> {
        if let Some(holder) = self
            .requests
            .lock()
            .map_err(ErrorCollection::locking_mutex)?
            .get_mut(&request_id)
        {
            *holder = RequestProcessorHolder::Ready(request_processor);
        } else {
            unreachable!();
        }

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
        let next_message = inbound.message().await?.and_then(|r| r.request_type);

        use crate::worker_service::worker::request_djikstra::RequestType::RequestId as ProtoRequestId;

        let request_id: RequestId = match next_message {
            Some(ProtoRequestId(id)) => id,
            _ => return Err(ErrorCollection::wrong_first_message()),
        };

        let mut request_processor = self.get_request_processor(request_id)?;
        request_processor.apply_update(&mut inbound).await?;

        // Move the processor in and out the task to satisfy borrow checker
        let (request_processor, result_vec) =
            tokio::task::spawn_blocking(move || request_processor.djikstra_step())
                .await
                .expect("RequestProcessor djikstra_step task panicked")?;

        self.put_request_processor(request_id, request_processor)?;
        let result_iter = result_vec.into_iter().map(Ok);
        let output = futures::stream::iter(result_iter);

        Ok(Response::new(Box::pin(output)))
    }
}

impl ErrorCollection {
    fn wrong_first_message() -> Status {
        Status::invalid_argument("First message in UpdateDjikstra stream must be request_id")
    }

    fn locking_mutex(e: impl std::error::Error) -> Status {
        Status::internal(format!("Internal error while locking mutex: {e}"))
    }

    fn duplicate_request() -> Status {
        Status::invalid_argument(
            "Error. Executer requested UpdateDjikstra, while another \
             UpdateDjikstra on this query was already pending",
        )
    }
}
