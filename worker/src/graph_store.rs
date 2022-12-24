pub use petgraph::graph::DefaultIx;

use petgraph::graph::Graph;
use std::collections::HashMap;

use tonic::Status;

pub type NodeId = u64;
pub type WorkerId = u32;
pub type EdgeWeight = u64;
pub type ShortestPathLen = u64;

#[derive(Debug)]
pub struct EdgeData {
    pub weight: EdgeWeight,
    pub worker_id: WorkerId,
}

pub type SPQGraph = Graph<NodeId, EdgeData>;
pub type NodeMapping = HashMap<NodeId, DefaultIx>;

pub trait IdIdxMapper {
    fn get_mapping(&self, id: NodeId) -> Result<DefaultIx, Status>;
}

impl IdIdxMapper for NodeMapping {
    fn get_mapping(&self, id: NodeId) -> Result<DefaultIx, Status> {
        self.get(&id)
            .copied()
            .ok_or(Status::invalid_argument(format!(
                "Cannot find node with id: {id}"
            )))
    }
}
