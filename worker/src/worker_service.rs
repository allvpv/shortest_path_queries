use std::pin::Pin;

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use generated::worker::request_djikstra;
use generated::worker::worker_server::Worker;
use generated::worker::{
    ArePresent, ForgetQueryMessage, NodeIds, RequestBacktrack, RequestDjikstra, ResponseBacktrack,
    ResponseDjikstra, RequestCoordinates, Coordinates
};

use crate::globals;
use crate::graph_store::{IdIdxMapper, SomeGraphMethods};
use crate::query_realizator;
use crate::ErrorCollection;

pub struct WorkerService {}

impl WorkerService {
    pub fn new() -> Self {
        WorkerService {}
    }
}

pub type RequestDjikstraStream = tonic::Streaming<RequestDjikstra>;
pub type ResponseDjikstraStream =
    Pin<Box<dyn Stream<Item = Result<ResponseDjikstra, Status>> + Send + 'static>>;
pub type ResponseBacktrackStream =
    Pin<Box<dyn Stream<Item = Result<ResponseBacktrack, Status>> + Send + 'static>>;

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

        let node_from_present = globals::mapping().contains_key(&node_from_id);
        let node_to_present = globals::mapping().contains_key(&node_to_id);

        Ok(Response::new(ArePresent {
            node_from_present,
            node_to_present,
        }))
    }

    async fn forget_query(
        &self,
        request: Request<ForgetQueryMessage>,
    ) -> Result<Response<()>, Status> {
        let ForgetQueryMessage { query_id } = request.into_inner();

        debug!("forgetting query[{query_id}]");
        globals::processor_holder().forget_query(query_id);

        Ok(Response::new(()))
    }

    type GetBacktrackStream = ResponseBacktrackStream;

    async fn get_backtrack(
        &self,
        request: Request<RequestBacktrack>,
    ) -> Result<Response<ResponseBacktrackStream>, Status> {
        let stream = query_realizator::get_backtrack_stream(request.into_inner());

        Ok(Response::new(Box::pin(stream)))
    }

    type UpdateDjikstraStream = ResponseDjikstraStream;

    async fn update_djikstra(
        &self,
        request: Request<RequestDjikstraStream>,
    ) -> Result<Response<ResponseDjikstraStream>, Status> {
        let mut inbound = request.into_inner();
        let next_message = inbound.message().await?.and_then(|r| r.message_type);

        use request_djikstra::MessageType::QueryData;

        let query_data = {
            if let Some(QueryData(data)) = next_message {
                data
            } else {
                return Err(ErrorCollection::wrong_first_message());
            }
        };

        debug!("got `update_djikstra` request: {query_data:?}");

        let processor = globals::processor_holder()
            .get_or_create(&query_data)
            .map_err(ErrorCollection::duplicated_request)?;

        let response = query_realizator::update_djikstra(&query_data, processor, inbound).await;

        match response {
            Ok(response) => Ok(Response::new(response)),
            Err(error) => {
                globals::processor_holder().forget_query(query_data.query_id);
                Err(error)
            }
        }
    }

    async fn get_node_coordinates(
        &self,
        request: Request<RequestCoordinates>,
    ) -> Result<Response<Coordinates>, Status> {
        let request  = request.into_inner();

        let node_idx = globals::mapping().get_mapping(request.node_id)?;
        let (lat, lon) = globals::graph().get_node(node_idx).coords;

        Ok(Response::new(Coordinates { lat, lon }))
    }
}

impl ErrorCollection {
    fn wrong_first_message() -> Status {
        Status::invalid_argument("first message in UpdateDjikstra stream must be query_id")
    }

    fn duplicated_request(e: impl std::error::Error) -> Status {
        Status::invalid_argument(format!(
            "executer requested UpdateDjikstra, while another UpdateDjikstra on this \
            query was already pending: {e}"
        ))
    }
}
