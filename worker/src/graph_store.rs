use std::collections::HashMap;

use tonic::Status;

pub type NodeId = u64;
pub type NodeIdx = u32;
pub type EdgeWeight = u64;
pub type ShortestPathLen = u64;

pub type WorkerId = u32;

#[derive(Debug)]
pub enum NodePointer {
    Domestic(NodeIdx),
    Foreign(NodeId, WorkerId),
}

#[derive(Debug)]
pub struct EdgePayload {
    pub weight: EdgeWeight,
    pub to: NodePointer,
}

#[derive(Debug)]
pub struct NodePayload {
    pub id: NodeId,
    pub coords: (f64, f64),
    pub edges: Vec<EdgePayload>,
}

pub type SPQGraph = Vec<NodePayload>;
pub type IdIdxMapping = HashMap<NodeId, NodeIdx>;

pub trait IdIdxMapper {
    fn get_mapping(&self, id: NodeId) -> Result<NodeIdx, Status>;
}

impl IdIdxMapper for IdIdxMapping {
    fn get_mapping(&self, id: NodeId) -> Result<NodeIdx, Status> {
        self.get(&id)
            .copied()
            .ok_or_else(|| Status::invalid_argument(format!("Cannot find node[id: {id}]")))
    }
}

pub trait SomeGraphMethods {
    fn get_node(&self, node: NodeIdx) -> &NodePayload;
    fn get_node_mut(&mut self, node: NodeIdx) -> &mut NodePayload;
    fn edges(&self, node: NodeIdx) -> std::slice::Iter<'_, EdgePayload>;
    fn add_node(&mut self, node_id: NodeId, coords: (f64, f64)) -> NodeIdx;
    fn add_edge(&mut self, from: NodeIdx, to: NodePointer, weight: EdgeWeight);
}

impl SomeGraphMethods for SPQGraph {
    fn get_node(&self, node: NodeIdx) -> &NodePayload {
        &self[node as usize]
    }

    fn get_node_mut(&mut self, node: NodeIdx) -> &mut NodePayload {
        &mut self[node as usize]
    }

    fn edges(&self, node: NodeIdx) -> std::slice::Iter<'_, EdgePayload> {
        self.get_node(node).edges.iter()
    }

    fn add_node(&mut self, node_id: NodeId, coords: (f64, f64)) -> NodeIdx {
        let node_idx = self.len() as NodeIdx;

        self.push(NodePayload {
            id: node_id,
            coords,
            edges: Vec::new(),
        });

        node_idx
    }

    fn add_edge(&mut self, from: NodeIdx, to: NodePointer, weight: EdgeWeight) {
        self.get_node_mut(from)
            .edges
            .push(EdgePayload { weight, to });
    }
}
