use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap};

use tonic::Status;

use generated::worker::request_djikstra;
use generated::worker::ResponseDjikstra;
use request_djikstra::QueryData;

use crate::globals;
use crate::graph_store::{IdIdxMapper, NodeId, NodeIdx, ShortestPathLen};
use crate::graph_store::{NodePointer, SomeGraphMethods, WorkerId};
use crate::proto_helpers;

pub type QueryId = u32;

#[derive(Debug, Clone, Copy)]
pub enum NodeParent {
    Root,
    Domestic(NodeIdx),
    Foreign(NodeId, WorkerId),
}

type ParentMap = HashMap<NodeId, NodeParent>;

#[derive(Debug)]
pub struct QueryProcessor {
    parent_map: ParentMap,
    queue: BinaryHeap<QueueElement>,
    smallest_foreign: Option<ShortestPathLen>,
    final_node: NodeId,
    query_id: QueryId,
}

pub enum StepResult {
    Remaining(Vec<ResponseDjikstra>),
    Finished(NodeId, ShortestPathLen),
}

impl QueryProcessor {
    pub fn query_id(&self) -> QueryId {
        self.query_id
    }

    pub fn new(data: &QueryData) -> Self {
        QueryProcessor {
            parent_map: ParentMap::new(),
            queue: BinaryHeap::new(),
            smallest_foreign: None,
            query_id: data.query_id,
            final_node: data.final_node_id,
        }
    }

    pub fn get_parent(&self, id: NodeId) -> Option<NodeParent> {
        self.parent_map.get(&id).copied()
    }

    pub fn update_smallest_foreign(&mut self, smallest_foreign: Option<ShortestPathLen>) {
        debug!(
            " -> replacing old smallest_foreign {:?} with new {:?}",
            self.smallest_foreign, smallest_foreign
        );

        self.smallest_foreign = smallest_foreign;
    }

    pub fn add_new_domestic_node(
        &mut self,
        id: NodeId,
        shortest: ShortestPathLen,
        parent: NodeParent,
    ) -> Result<(), Status> {
        let idx = globals::mapping().get_mapping(id)?;
        let entry = self.parent_map.entry(id);

        debug!("new domestic node[id: {id}, idx: {idx}, len: {shortest}, parent: {parent:?}]");

        match entry {
            Entry::Occupied(_) => debug!(" -> node[id: {id}] was already visited"),
            Entry::Vacant(entry) => {
                debug!(" -> node[id: {id}] is not visited: pushing to queue");
                entry.insert(parent);
                self.queue.push(QueueElement { idx, shortest });
            }
        }

        Ok(())
    }

    // To be executed on the blocking thread
    pub fn djikstra_step(mut self) -> Result<(Self, StepResult), Status> {
        type RVec = Vec<ResponseDjikstra>;
        let mut responses = RVec::new();

        let append_response_foreign =
            |responses: &mut RVec, node_id, worker_id, parent_id, shortest| {
                responses.push(proto_helpers::new_foreign_node(
                    node_id, worker_id, parent_id, shortest,
                ));
            };

        let append_response_domestic = |responses: &mut RVec, shortest| {
            debug!("pushing smallest domestic node[len: {shortest}] to response");
            responses.push(proto_helpers::domestic_smallest_node(shortest));
        };

        let check_success = |node_id, shortest| {
            if self.final_node == node_id {
                debug!("success: node: {node_id}, length: {shortest}");
                Some(StepResult::Finished(node_id, shortest))
            } else {
                None
            }
        };

        // We consumed all nodes from our graph fragment? Time to stop the query.
        while let Some(node) = self.queue.peek() {
            // Smallest node does not belong to this worker? Time to stop the query.
            if let Some(smf) = self.smallest_foreign {
                if smf < node.shortest {
                    debug!(
                        "smallest node does not belong to this worker, {} vs {}",
                        node.shortest, smf
                    );
                    append_response_domestic(&mut responses, node.shortest);
                    break;
                }
            }

            let node = self.queue.pop().unwrap();

            for edge in globals::graph().edges(node.idx) {
                let new_node_id = match edge.to {
                    NodePointer::Foreign(node_id, _) => node_id,
                    NodePointer::Domestic(new_node_idx) => {
                        globals::graph().get_node(new_node_idx).id
                    }
                };

                let parent_idx = node.idx;

                match self.parent_map.entry(new_node_id) {
                    Entry::Occupied(_) => continue, // Already visited.
                    Entry::Vacant(entry) => entry.insert(NodeParent::Domestic(parent_idx)),
                };

                let new_shortest = node.shortest + edge.weight;

                // Maybe we found the final node?
                if let Some(success) = check_success(new_node_id, new_shortest) {
                    return Ok((self, success));
                }

                match edge.to {
                    NodePointer::Foreign(_, worker_id) => {
                        let parent_id = globals::graph().get_node(parent_idx).id;

                        append_response_foreign(
                            &mut responses,
                            new_node_id,
                            worker_id,
                            parent_id,
                            new_shortest,
                        );

                        if let Some(smallest) = self.smallest_foreign.as_mut() {
                            *smallest = std::cmp::min(*smallest, new_shortest);
                        } else {
                            self.smallest_foreign = Some(new_shortest);
                        }
                    }
                    NodePointer::Domestic(new_node_idx) => {
                        self.queue
                            .push(QueueElement::new(new_node_idx, new_shortest));
                    }
                }
            }
        }

        Ok((self, StepResult::Remaining(responses)))
    }
}

#[derive(Eq, PartialEq, Debug)]
struct QueueElement {
    idx: NodeIdx,
    shortest: ShortestPathLen,
}

impl QueueElement {
    fn new(idx: NodeIdx, shortest: ShortestPathLen) -> Self {
        QueueElement { idx, shortest }
    }
}

impl Ord for QueueElement {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note `reverse`: smallest element on top
        self.shortest.cmp(&other.shortest).reverse()
    }
}

impl PartialOrd for QueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
