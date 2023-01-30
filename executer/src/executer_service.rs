use std::pin::Pin;
use futures::Stream;
use tonic::{Request, Response, Result, Status};

use crate::globals;

use generated::executer;
use generated::executer::executer_server::Executer;

pub struct ExecuterService {}

type NodeStream = Pin<Box<dyn Stream<Item = Result<executer::Node, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl Executer for ExecuterService {
    async fn shortest_path_query(
        &self,
        request: Request<executer::QueryData>,
    ) -> Result<Response<executer::QueryResults>> {
        let response = globals::queries_manager()
            .shortest_path_query(request.into_inner())
            .await?;

        Ok(Response::new(response))
    }

    type BacktrackPathForQueryStream = NodeStream;

    async fn backtrack_path_for_query(
        &self,
        request: Request<executer::QueryId>,
    ) -> Result<Response<NodeStream>> {
        let query_id = request.into_inner().query_id;
        let stream = globals::queries_manager().get_backtrack_stream(query_id);

        Ok(Response::new(Box::pin(stream) as NodeStream))
    }

    async fn forget_query(&self, request: Request<executer::QueryId>) -> Result<Response<()>> {
        globals::queries_manager()
            .forget_query(request.into_inner())
            .await?;

        Ok(Response::new(()))
    }
}
