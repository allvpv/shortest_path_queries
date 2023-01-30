use futures::Stream;
use tonic::Status;

use crate::globals;
use crate::graph_store;
use crate::proto_helpers;
use crate::query_processor;
use crate::worker_service;

use crate::graph_store::SomeGraphMethods;
use crate::ErrorCollection;

use generated::worker;

async fn apply_update(
    data: &worker::request_djikstra::QueryData,
    processor: &mut query_processor::QueryProcessor,
    inbound: &mut worker_service::RequestDjikstraStream,
) -> Result<(), Status> {
    processor.update_smallest_foreign(data.smallest_foreign_node);

    while let Some(message) = inbound.message().await? {
        use query_processor::NodeParent;
        use worker::request_djikstra::MessageType::{NewDomesticNode, QueryData};

        match message.message_type {
            Some(NewDomesticNode(node)) => {
                let parent = match node.parent_node {
                    None => NodeParent::Root,
                    Some(node) => NodeParent::Foreign(node.node_id, node.worker_id),
                };

                processor.add_new_domestic_node(node.node_id, node.shortest_path_len, parent)?;
            }
            Some(QueryData(_)) => return Err(ErrorCollection::duplicated_query_data()),
            None => break,
        }
    }

    Ok(())
}

pub async fn update_djikstra(
    data: &worker::request_djikstra::QueryData,
    mut processor: query_processor::QueryProcessor,
    mut inbound: worker_service::RequestDjikstraStream,
) -> Result<worker_service::ResponseDjikstraStream, Status> {
    apply_update(data, &mut processor, &mut inbound).await?;

    // Move the processor in and out the task to satisfy the borrow checker
    let (processor, result) = tokio::task::spawn_blocking(move || processor.djikstra_step())
        .await
        .expect("QueryProcessor djikstra_step task panicked")?;

    use query_processor::StepResult::{Finished, Remaining};

    let output = match result {
        Finished(node_id, shortest) => {
            info!(
                "finished with success (node[id {}, len: {}])",
                node_id, shortest
            );
            globals::processor_holder().forget_query(data.query_id);

            let message = proto_helpers::success(node_id, shortest);
            let stream = futures::stream::once(async { Ok(message) });

            Box::pin(stream) as worker_service::ResponseDjikstraStream
        }
        Remaining(responses) => {
            info!("forwarding the request to the executer");
            globals::processor_holder().put_back_query(processor);

            let messages = responses.into_iter().map(Ok);
            let stream = futures::stream::iter(messages);

            Box::pin(stream) as worker_service::ResponseDjikstraStream
        }
    };

    Ok(output)
}

pub fn get_backtrack_stream(
    request: worker::RequestBacktrack,
) -> impl Stream<Item = Result<worker::ResponseBacktrack, Status>> + Send + 'static {
    debug!("GetBacktrack request: {request:?}");

    let worker::RequestBacktrack {
        query_id,
        from_node,
    } = request;

    async_stream::try_stream! {
        use worker::ResponseBacktrack;
        use query_processor::NodeParent;

        let processor = globals::processor_holder()
            .get_existing(query_id)?
            .ok_or_else(|| ErrorCollection::query_not_found(query_id))?;

        let mut next_node = Some(from_node);

        let map_idx_to_id = |idx| -> graph_store::NodeId {
            globals::graph().get_node(idx).id
        };

        while let Some(current_node) = next_node {
            let parent = processor
                .get_parent(current_node)
                .ok_or_else(|| ErrorCollection::cannot_find_parent(current_node));

            if let Err(_) = parent {
                globals::processor_holder().put_back_query(processor);
                parent?; // Due to the limitations of `try_stream!`
                return;
            }

            let parent = parent?;

            next_node = match parent {
                NodeParent::Root => None,
                NodeParent::Foreign(id, worker) => {
                    yield ResponseBacktrack { node_id: id, worker_id: Some(worker) };
                    None
                }
                NodeParent::Domestic(idx) => {
                    yield ResponseBacktrack { node_id: current_node, worker_id: None };
                    Some(map_idx_to_id(idx))
                }
            }
        }

        globals::processor_holder().put_back_query(processor);
    }
}

impl ErrorCollection {
    fn cannot_find_parent(node_id: graph_store::NodeId) -> Status {
        Status::not_found(format!("cannot find parent of node[idx: {node_id}]"))
    }

    fn duplicated_query_data() -> Status {
        Status::invalid_argument("duplicated QueryData in the middle of the stream")
    }

    fn query_not_found(query_id: query_processor::QueryId) -> Status {
        Status::invalid_argument(format!("query[id: {query_id}] not found"))
    }
}
