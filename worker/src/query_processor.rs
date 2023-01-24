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
        println!(
            " -> replacing old smallest_foreign {:?} with new {:?}",
            self.smallest_foreign, smallest_foreign
        );

        self.smallest_foreign = smallest_foreign;
    }

    pub fn add_new_domestic_node(
        &mut self,
        id: NodeId,
        shortest: ShortestPathLen,
    ) -> Result<(), Status> {
        let not_visited = self.visited.replace(id).is_none();

        println!("new domestic node; node_id: {id}, len: {shortest}");

        if not_visited {
            let idx = self.mapping.get_mapping(id)?;
            println!("node (id {id}, idx {idx}) is not visited: pushing to queue");
            self.queue.push(QueueElement { idx, shortest });
        } else {
            println!("node (id {id}) was already visited");
        }

        Ok(())
    }

    // To be executed on the blocking thread
    pub fn djikstra_step(mut self) -> Result<(Self, StepResult), Status> {
        type RVec = Vec<ResponseDjikstra>;
        let mut responses = RVec::new();

        let append_response_foreign = |responses: &mut RVec, node_id, worker_id, shortest| {
            println!(
                "pushing new foreign node[id: {}, len: {}] from worker[id: {}] to response",
                node_id, shortest, worker_id
            );

            responses.push(proto_helpers::new_foreign_node(
                node_id, worker_id, shortest,
            ));
        };

        let append_response_domestic = |responses: &mut RVec, shortest| {
            println!(
                "pushing smallest domestic node[len: {}] to response",
                shortest
            );

            responses.push(proto_helpers::domestic_smallest_node(shortest));
        };

        let check_success = |node_id, shortest| {
            if self.final_node == node_id {
                println!("success!, node: {} length: {}", node_id, shortest);
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
                    println!(
                        "smallest node does not belong to this worker, {} vs {}",
                        node.shortest, smf
                    );
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
        // Note `reverse`: smallest element on top
        self.shortest.cmp(&other.shortest).reverse()
    }
}

impl PartialOrd for QueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
