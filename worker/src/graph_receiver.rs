pub mod manager {
    tonic::include_proto!("manager");
}

use tonic::transport::Channel;
use tonic::{Request, Status};

use manager::graph_piece;
use manager::manager_service_client::ManagerServiceClient;

use crate::graph_store;

use graph_store::IdIdxMapper;

pub struct GraphReceiver {
    pub client: ManagerServiceClient<Channel>,
    pub worker_id: graph_store::WorkerId,
    pub graph: graph_store::SPQGraph,
    pub mapping: graph_store::NodeMapping,
}

impl GraphReceiver {
    pub async fn new(mut client: ManagerServiceClient<Channel>) -> Result<Self, tonic::Status> {
        let response = client.register_worker(Request::new(())).await?;
        let worker_id = response.get_ref().worker_id;

        Ok(GraphReceiver {
            client,
            worker_id,
            graph: graph_store::SPQGraph::new(),
            mapping: graph_store::NodeMapping::new(),
        })
    }

    pub async fn receive_graph(&mut self) -> Result<(), Status> {
        let mut stream = self
            .client
            .get_graph_fragment(Request::new(()))
            .await?
            .into_inner();

        while let Some(response) = stream.message().await? {
            use graph_piece::GraphElement::{Edges, Nodes};
            use graph_store::{DefaultIx, EdgeData};

            match response.graph_element {
                Some(Nodes(node)) => {
                    let node_id = node.node_id;
                    let node_idx = self.graph.add_node(node_id).index() as DefaultIx;

                    self.mapping.insert(node_id, node_idx);
                }
                Some(Edges(edge)) => {
                    let node_from_id = self.mapping.get_mapping(edge.node_from_id)?;
                    let node_to_id = self.mapping.get_mapping(edge.node_to_id)?;

                    self.graph.add_edge(
                        node_from_id.into(),
                        node_to_id.into(),
                        EdgeData {
                            weight: edge.weight,
                            worker_id: edge.node_to_worker_id.unwrap_or(self.worker_id),
                        },
                    );
                }
                None => {
                    eprintln!("Warning: Got empty GraphPiece with no node or edge");
                }
            }
        }

        Ok(())
    }
}
