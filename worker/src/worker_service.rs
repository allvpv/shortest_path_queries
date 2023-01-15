use std::pin::Pin;

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use generated::worker::worker_server::Worker;
use generated::worker::{ArePresent, NodeIds, RequestDjikstra, ResponseDjikstra};

use crate::graph_store::{IdIdxMapping, SPQGraph};
use crate::proto_helpers;
use crate::query_processor::QueryProcessor;
use crate::query_processor::StepResult::{Finished, Remaining};
use crate::query_processor_holder::QueryProcessorHolder;
use crate::ErrorCollection;

use generated::worker::request_djikstra;

#[derive(Debug)]
pub struct WorkerService {
    processors: QueryProcessorHolder,
}

impl WorkerService {
    pub fn new(graph: SPQGraph, mapping: IdIdxMapping) -> Self {
        WorkerService {
            processors: QueryProcessorHolder::new(graph, mapping),
        }
    }
}

type RequestDjikstraStream = tonic::Streaming<RequestDjikstra>;
type ResponseDjikstraStream =
    Pin<Box<dyn Stream<Item = Result<ResponseDjikstra, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn are_nodes_present(
        &self,
        request: Request<NodeIds>,
    ) -> Result<Response<ArePresent>, Status> {
        let NodeIds {
            node_from_id,
            node_to_id,
        } = request.into_inner();

        let node_from_present = self.processors.get_mapping().contains_key(&node_from_id);
        let node_to_present = self.processors.get_mapping().contains_key(&node_to_id);

        Ok(Response::new(ArePresent {
            node_from_present,
            node_to_present,
        }))
    }

    type UpdateDjikstraStream = ResponseDjikstraStream;

    async fn update_djikstra(
        &self,
        request: Request<RequestDjikstraStream>,
    ) -> Result<Response<ResponseDjikstraStream>, Status> {
        let mut inbound = request.into_inner();
        let next_message = inbound.message().await?.and_then(|r| r.message_type);

        use request_djikstra::MessageType::QueryData;

        debug!("applying update");

        let query_data = {
            if let Some(QueryData(data)) = next_message {
                data
            } else {
                return Err(ErrorCollection::wrong_first_message());
            }
        };

        debug!(" -> query data: {query_data:?}");

        let mut processor = self
            .processors
            .get_for_query(&query_data)
            .map_err(ErrorCollection::duplicated_request)?;

        processor.update_smallest_foreign(query_data.smallest_foreign_node);

        Self::apply_update(&mut processor, &mut inbound).await?;

        // Move the processor in and out the task to satisfy the borrow checker
        let (processor, result) = tokio::task::spawn_blocking(move || processor.djikstra_step())
            .await
            .expect("QueryProcessor djikstra_step task panicked")?;

        let output: ResponseDjikstraStream = match result {
            Finished(node_id, shortest) => {
                info!("finished with success (node[id {}, len: {}])", node_id, shortest);

                self.processors.forget_query(processor)?;
                let message = proto_helpers::success(node_id, shortest);
                Box::pin(futures::stream::once(async { Ok(message) }))
            }
            Remaining(responses) => {
                info!("forwarding the request to the executer");

                self.processors.put_back_query(processor)?;
                let messages = responses.into_iter().map(Ok);
                Box::pin(futures::stream::iter(messages))
            }
        };

        Ok(Response::new(output))
    }
}

impl WorkerService {
    // Applies updates for this query
    async fn apply_update(
        processor: &mut QueryProcessor,
        inbound: &mut RequestDjikstraStream,
    ) -> Result<(), Status> {
        while let Some(message) = inbound.message().await? {
            use request_djikstra::MessageType::{NewDomesticNode, QueryData};

            match message.message_type {
                Some(NewDomesticNode(node)) => {
                    processor.add_new_domestic_node(node.node_id, node.shortest_path_len)?;
                }
                Some(QueryData(_)) => return Err(ErrorCollection::duplicated_query_data()),
                None => break,
            }
        }

        Ok(())
    }
}

impl ErrorCollection {
    fn wrong_first_message() -> Status {
        Status::invalid_argument("First message in UpdateDjikstra stream must be query_id")
    }

    fn duplicated_request(e: impl std::error::Error) -> Status {
        Status::invalid_argument(format!(
            "Executer requested UpdateDjikstra, while another UpdateDjikstra on this \
                    query was already pending: {e}"
        ))
    }

    fn duplicated_query_data() -> Status {
        Status::invalid_argument("Duplicated QueryData in the middle of the stream")
    }
}
