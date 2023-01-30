use tonic::transport::Channel;
use tonic::{Request, Status};

use generated::manager::manager_service_client::ManagerServiceClient;
use generated::manager::{graph_piece, WorkerMetadata, WorkerProperties};

use crate::globals;
use crate::graph_store;

use graph_store::{IdIdxMapper, IdIdxMapping, NodePointer, SPQGraph, SomeGraphMethods, WorkerId};

pub struct GraphReceiver {
    pub client: ManagerServiceClient<Channel>,
    pub worker_id: WorkerId,
}

impl GraphReceiver {
    pub async fn new(
        mut client: ManagerServiceClient<Channel>,
        listening_address: String,
    ) -> Result<Self, tonic::Status> {
        debug!("registering this worker in manager");

        let response = client
            .register_worker(Request::new(WorkerProperties { listening_address }))
            .await?;
        let worker_id = response.get_ref().worker_id;

        debug!("worker_id has been assigned: {worker_id}");

        Ok(GraphReceiver { client, worker_id })
    }

    pub async fn receive_graph(&mut self) -> Result<(), Status> {
        info!("requesting graph");

        let mut graph = SPQGraph::new();
        let mut mapping = IdIdxMapping::new();
        let mut stream = self
            .client
            .get_graph_fragment(Request::new(WorkerMetadata {
                worker_id: self.worker_id,
            }))
            .await?
            .into_inner();

        while let Some(response) = stream.message().await? {
            use graph_piece::GraphElement::{Edges, Nodes};

            match response.graph_element {
                Some(Nodes(node)) => {
                    let node_idx = graph.add_node(node.node_id);
                    mapping.insert(node.node_id, node_idx);

                    debug!("got node[id: {}, idx: {}]", node.node_id, node_idx)
                }
                Some(Edges(edge)) => {
                    let node_from_idx = mapping.get_mapping(edge.node_from_id)?;
                    // If `worker_id` is present, then the edge points to foreign node that belongs
                    // to some other worker
                    let pointer_to = match edge.node_to_worker_id {
                        Some(worker_id) => NodePointer::Foreign(edge.node_to_id, worker_id),
                        None => NodePointer::Domestic(mapping.get_mapping(edge.node_to_id)?),
                    };

                    debug!(
                        "got edge[from_node_id: {}, to_node_id: {}, \
                        weight: {}, from_idx: {}, pointer: {:?}]",
                        edge.node_from_id, edge.node_to_id, edge.weight, node_from_idx, pointer_to
                    );

                    graph.add_edge(node_from_idx, pointer_to, edge.weight);
                }
                None => {
                    warn!("got empty GraphPiece with no node or edge!");
                }
            }
        }

        debug!("finished receiving graph");

        globals::GRAPH
            .set(graph)
            .map_err(|_| Status::internal(format!("cannot set global graph")))?;

        globals::MAPPING
            .set(mapping)
            .map_err(|_| Status::internal(format!("cannot set global mapping")))?;

        Ok(())
    }
}
