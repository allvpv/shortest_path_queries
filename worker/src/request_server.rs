use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use tonic::Status;

use crate::graph_store::{DefaultIx, IdIdxMapper, NodeId, NodeMapping, SPQGraph, ShortestPathLen};
use crate::worker_service::worker::{RequestDjikstra, ResponseDjikstra};

pub type RequestId = u32;

#[derive(Debug)]
pub struct RequestServer {
    graph: Arc<SPQGraph>,
    mapping: Arc<NodeMapping>,
    visited: HashSet<NodeId>,
    queue: BinaryHeap<QueueElement>,
    smallest_foreign: Option<ShortestPathLen>,
}

#[derive(Eq, PartialEq, Debug)]
struct QueueElement {
    ix: DefaultIx,
    sp_len: ShortestPathLen,
}

impl Ord for QueueElement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sp_len.cmp(&other.sp_len)
    }
}

impl PartialOrd for QueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl RequestServer {
    pub fn new(graph: Arc<SPQGraph>, mapping: Arc<NodeMapping>) -> Self {
        RequestServer {
            graph,
            mapping,
            visited: HashSet::new(),
            queue: BinaryHeap::new(),
            smallest_foreign: None,
        }
    }

    // Applies updates for this RequestID worker 
    pub async fn apply_update(
        &mut self,
        inbound: &mut tonic::codec::Streaming<RequestDjikstra>,
    ) -> Result<(), Status> {
        self.smallest_foreign = None;

        while let Some(request) = inbound.message().await? {
            use crate::worker_service::worker::request_djikstra::RequestType::{
                NewMyEl, RequestId as ProtoRequestId, SmallestForeignEl,
            };

            match request.request_type {
                Some(NewMyEl(element)) => {
                    let not_visited = self.visited.replace(element.node_id).is_none();

                    if not_visited {
                        self.queue.push(QueueElement {
                            ix: self.mapping.get_mapping(element.node_id)?,
                            sp_len: element.shortest_path_len,
                        });
                    }
                }
                Some(SmallestForeignEl(foreign)) => {
                    self.smallest_foreign = Some(foreign.shortest_path_len);
                }
                Some(ProtoRequestId(id)) => {
                    return Err(Status::invalid_argument(format!(
                        "Duplicated RequestId {id} in the middle of the stream"
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
                let next_element = &graph[edge.target()].;

                if self.visited.replace(next_element).is_some() {
                    continue;
                }

                let
                let next_element_weight = edge.weight() + node.sp_len;

            }
        }
        */
    }
}
