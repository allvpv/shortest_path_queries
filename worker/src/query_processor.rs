use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use tonic::Status;

use crate::graph_store::{IdIdxMapper, IdIdxMapping, NodeId, NodeIdx, SPQGraph, ShortestPathLen};
use crate::worker_service::worker::{RequestDjikstra, ResponseDjikstra};

pub type QueryId = u32;

#[derive(Debug)]
pub struct QueryProcessor {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    visited: HashSet<NodeId>,
    queue: BinaryHeap<QueueElement>,
    smallest_foreign: Option<ShortestPathLen>,
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
    pub fn new(graph: Arc<SPQGraph>, mapping: Arc<IdIdxMapping>) -> Self {
        QueryProcessor {
            graph,
            mapping,
            visited: HashSet::new(),
            queue: BinaryHeap::new(),
            smallest_foreign: None,
        }
    }

    // Applies updates for this query
    pub async fn apply_update(
        &mut self,
        inbound: &mut tonic::codec::Streaming<RequestDjikstra>,
    ) -> Result<(), Status> {
        self.smallest_foreign = None;

        while let Some(message) = inbound.message().await? {
            use crate::worker_service::worker::request_djikstra::MessageType::{
                NewMyEl, QueryId as ProtoQueryId, SmallestForeignEl,
            };

            match message.message_type {
                Some(NewMyEl(element)) => {
                    let not_visited = self.visited.replace(element.node_id).is_none();

                    if not_visited {
                        self.queue.push(QueueElement {
                            idx: self.mapping.get_mapping(element.node_id)?,
                            shortest: element.shortest_path_len,
                        });
                    }
                }
                Some(SmallestForeignEl(foreign)) => {
                    self.smallest_foreign = Some(foreign.shortest_path_len);
                }
                Some(ProtoQueryId(id)) => {
                    return Err(Status::invalid_argument(format!(
                        "Duplicated QueryId {id} in the middle of the stream"
                    )))
                }
                None => break,
            }
        }

        Ok(())
    }

    // To be executed on the blocking thread
    pub fn djikstra_step(self) -> Result<(Self, Vec<ResponseDjikstra>), Status> {
        // TODO
        Ok((self, Vec::new()))

        /*
        let responses = Vec::<ResponseDjikstra>::new();

        while Some(node) = self.queue.peek() {
            if let Some(smallest_foreign) = self.smallest_foreign {
                if smallest_foreign < node.sp_len {
                    break;
                }
            }

            for node in self.graph.edges(node.ix.into()) {
                let next_element = &graph[edge.target()];

                if self.visited.replace(next_element).is_some() {
                    continue;
                }

                let next_sp_len = edge.weight() + node.sp_len;
            }
        }
        */
    }
}
