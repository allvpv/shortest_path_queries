use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use tonic::Status;

use crate::graph_store::{IdIdxMapper, IdIdxMapping, NodeId, NodeIdx, SPQGraph, ShortestPathLen};
use crate::graph_store::{NodePointer, SomeGraphMethods, VisitedMap};
use crate::worker_service::worker::{request_djikstra, response_djikstra};
use crate::worker_service::worker::{RequestDjikstra, ResponseDjikstra};

pub type QueryId = u32;

#[derive(Debug)]
pub struct QueryProcessor {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    visited: VisitedMap,
    queue: BinaryHeap<QueueElement>,
    smallest_foreign: Option<ShortestPathLen>,
    final_node: NodeId,
}

#[derive(Eq, PartialEq, Debug)]
struct QueueElement {
    idx: NodeIdx,
    shortest: ShortestPathLen,
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

impl QueryProcessor {
    fn update_smallest_foreign(
        smallest_foreign: &mut Option<ShortestPathLen>,
        new_foreign: ShortestPathLen,
    ) {
        if let Some(smallest) = smallest_foreign.as_mut() {
            *smallest = std::cmp::min(*smallest, new_foreign);
        } else {
            *smallest_foreign = Some(new_foreign);
        }
    }

    pub fn new(graph: Arc<SPQGraph>, mapping: Arc<IdIdxMapping>, final_node: NodeId) -> Self {
        QueryProcessor {
            graph,
            mapping,
            visited: HashSet::new(),
            queue: BinaryHeap::new(),
            smallest_foreign: None,
            final_node,
        }
    }

    // Applies updates for this query
    pub async fn apply_update(
        &mut self,
        inbound: &mut tonic::codec::Streaming<RequestDjikstra>,
        smallest_foreign: Option<ShortestPathLen>,
    ) -> Result<(), Status> {
        self.smallest_foreign = smallest_foreign;

        while let Some(message) = inbound.message().await? {
            use request_djikstra::MessageType::{NewDomesticNode, QueryData};

            match message.message_type {
                Some(NewDomesticNode(node)) => {
                    let not_visited = self.visited.replace(node.node_id).is_none();

                    if not_visited {
                        self.queue.push(QueueElement {
                            idx: self.mapping.get_mapping(node.node_id)?,
                            shortest: node.shortest_path_len,
                        });
                    }
                }
                Some(QueryData(_)) => {
                    return Err(Status::invalid_argument(
                        "Duplicated QueryData in the middle of the stream",
                    ));
                }
                None => break,
            }
        }

        Ok(())
    }

    fn check_for_success(
        &self,
        node_id: NodeId,
        shortest: ShortestPathLen,
    ) -> Option<Vec<ResponseDjikstra>> {
        if self.final_node == node_id {
            use response_djikstra::MessageType::Success as SuccessVariant;
            use response_djikstra::Success;

            Some(vec![ResponseDjikstra {
                message_type: Some(SuccessVariant(Success {
                    node_id,
                    shortest_path_len: shortest,
                })),
            }])
        } else {
            None
        }
    }

    // To be executed on the blocking thread
    pub fn djikstra_step(mut self) -> Result<(Self, Vec<ResponseDjikstra>), Status> {
        let mut responses = Vec::<ResponseDjikstra>::new();

        // We consumed all nodes from our graph fragment? Time to stop the query.
        while let Some(node) = self.queue.peek() {
            // Smallest node does not belong to this worker? Time to stop the query.
            if let Some(smf) = self.smallest_foreign {
                if smf < node.shortest {
                    use response_djikstra::{
                        MessageType::SmallestDomesticNode as Variant, SmallestDomesticNode,
                    };

                    responses.push(ResponseDjikstra {
                        message_type: Some(Variant(SmallestDomesticNode {
                            shortest_path_len: node.shortest,
                        })),
                    });

                    break;
                }
            }

            let node = self.queue.pop().unwrap();

            for edge in self.graph.edges(node.idx) {
                match edge.to {
                    NodePointer::Foreign(new_node_id, worker_id) => {
                        // Already visited.
                        if self.visited.replace(new_node_id).is_some() {
                            continue;
                        }

                        let new_shortest = node.shortest + edge.weight;

                        // Maybe we found the final node?
                        if let Some(success) = self.check_for_success(new_node_id, new_shortest) {
                            return Ok((self, success));
                        }

                        use response_djikstra::{
                            MessageType::NewForeignNode as Variant, NewForeignNode,
                        };

                        // New foreign node visited.
                        responses.push(ResponseDjikstra {
                            message_type: Some(Variant(NewForeignNode {
                                node_id: new_node_id,
                                worker_id,
                                shortest_path_len: new_shortest,
                            })),
                        });

                        Self::update_smallest_foreign(&mut self.smallest_foreign, new_shortest);
                    }
                    NodePointer::Domestic(new_node_idx) => {
                        let new_node = self.graph.get_node(new_node_idx);

                        // Already visited.
                        if self.visited.replace(new_node.id).is_some() {
                            continue;
                        }

                        let new_shortest = node.shortest + edge.weight;

                        // Maybe we found the final node?
                        if let Some(success) = self.check_for_success(new_node.id, new_shortest) {
                            return Ok((self, success));
                        }

                        self.queue.push(QueueElement {
                            idx: new_node_idx,
                            shortest: new_shortest,
                        });
                    }
                }
            }
        }

        Ok((self, responses))
    }
}
