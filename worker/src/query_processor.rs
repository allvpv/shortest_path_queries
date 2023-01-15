use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use tonic::Status;

use crate::graph_store::{IdIdxMapper, IdIdxMapping, NodeId, NodeIdx, SPQGraph, ShortestPathLen};
use crate::graph_store::{NodePointer, SomeGraphMethods, VisitedMap};
use crate::proto_helpers;

use generated::worker::request_djikstra;
use generated::worker::ResponseDjikstra;
use request_djikstra::QueryData;

pub type QueryId = u32;

#[derive(Debug)]
pub struct QueryProcessor {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    visited: VisitedMap,
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

    pub fn new(graph: Arc<SPQGraph>, mapping: Arc<IdIdxMapping>, query_data: &QueryData) -> Self {
        QueryProcessor {
            graph,
            mapping,
            visited: HashSet::new(),
            queue: BinaryHeap::new(),
            smallest_foreign: None,
            query_id: query_data.query_id,
            final_node: query_data.final_node_id,
        }
    }

    pub fn update_smallest_foreign(&mut self, smallest_foreign: Option<ShortestPathLen>) {
        self.smallest_foreign = smallest_foreign;
    }

    pub fn add_new_domestic_node(
        &mut self,
        id: NodeId,
        shortest: ShortestPathLen,
    ) -> Result<(), Status> {
        let not_visited = self.visited.replace(id).is_none();

        println!("New domestic node; node_id: {id}, len: {shortest}");

        if not_visited {
            println!("Node is not visited, pushing to queue");
            let idx = self.mapping.get_mapping(id)?;
            println!("Node id is: {id}; idx: {idx}");

            self.queue.push(QueueElement { idx, shortest });
        }

        Ok(())
    }

    // To be executed on the blocking thread
    pub fn djikstra_step(mut self) -> Result<(Self, StepResult), Status> {
        type RVec = Vec<ResponseDjikstra>;
        let mut responses = RVec::new();

        let append_response_foreign = |responses: &mut RVec, node_id, worker_id, shortest| {
            responses.push(proto_helpers::new_foreign_node(
                node_id, worker_id, shortest,
            ));
        };

        let append_response_domestic = |responses: &mut RVec, shortest| {
            responses.push(proto_helpers::domestic_smallest_node(shortest));
        };

        let check_success = |node_id, shortest| {
            if self.final_node == node_id {
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
                    append_response_domestic(&mut responses, node.shortest);
                    break;
                }
            }

            let node = self.queue.pop().unwrap();

            for edge in self.graph.edges(node.idx) {
                let new_node_id = match edge.to {
                    NodePointer::Foreign(node_id, _) => node_id,
                    NodePointer::Domestic(new_node_idx) => self.graph.get_node(new_node_idx).id,
                };

                // Already visited.
                if self.visited.replace(new_node_id).is_some() {
                    continue;
                }

                let new_shortest = node.shortest + edge.weight;

                // Maybe we found the final node?
                if let Some(success) = check_success(new_node_id, new_shortest) {
                    return Ok((self, success));
                }

                match edge.to {
                    NodePointer::Foreign(_, worker_id) => {
                        append_response_foreign(
                            &mut responses,
                            new_node_id,
                            worker_id,
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
        self.shortest.cmp(&other.shortest)
    }
}

impl PartialOrd for QueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
