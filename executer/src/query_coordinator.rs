use async_stream::AsyncStream;
use futures::stream::FuturesUnordered;
use futures::Future;
use futures::TryFutureExt;
use futures::TryStreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tonic::Result;
use tonic::Status;

use generated::executer::QueryFinished;
use generated::worker::request_djikstra;
use generated::worker::response_djikstra;
use generated::worker::worker_client::WorkerClient;
use generated::worker::RequestDjikstra;

use crate::executer_service::{NodeId, ShortestPathLen};
use crate::workers_connection::Worker;
use crate::workers_connection::WorkerId;
use crate::ErrorCollection;

use request_djikstra::NewDomesticNode;
use response_djikstra::MessageType;

type WorkerIdx = usize;

struct WorkerExtended {
    id: WorkerId,
    channel: WorkerClient<Channel>,
    minimal: Option<NodeId>,
    new_nodes: Vec<NewDomesticNode>,
}

impl WorkerExtended {
    fn from(worker: &Worker) -> Self {
        WorkerExtended {
            id: worker.id,
            channel: worker.channel.clone(), // Cloning `Channel` is cheap (and unavoidable I guess)
            minimal: None,
            new_nodes: Vec::new(),
        }
    }

    fn push_new_domestic(&mut self, node_id: NodeId, shortest_path_len: ShortestPathLen) {
        self.minimal = std::cmp::min(self.minimal, Some(shortest_path_len));
        self.new_nodes.push(NewDomesticNode {
            node_id,
            shortest_path_len,
        });
    }

    fn extract_new_domestic(&mut self) -> std::vec::Vec<NewDomesticNode> {
        std::mem::take(&mut self.new_nodes)
    }
}

pub struct QueryCoordinator {
    workers: Vec<WorkerExtended>,
    query_id: u32,

    node_id_from: NodeId,
    node_id_to: NodeId,

    first_worker_idx: WorkerIdx,
    last_worker_idx: WorkerIdx,

    global_shortest: Option<(WorkerIdx, ShortestPathLen)>,
}

impl QueryCoordinator {
    fn update_shortest(&mut self, worker: WorkerIdx, shortest: ShortestPathLen) {
        match self.global_shortest.as_mut() {
            None => self.global_shortest = Some((worker, shortest)),
            Some((worker_, shortest_)) => {
                if shortest < *shortest_ {
                    *worker_ = worker;
                    *shortest_ = shortest;
                }
            }
        }
    }

    fn find_shortest_foreign(&self, current: WorkerIdx) -> Option<ShortestPathLen> {
        self.workers
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != current)
            .filter_map(|(_, w)| w.minimal)
            .min()
    }

    pub async fn new(workers: &[Worker], from: NodeId, to: NodeId, query_id: u32) -> Result<Self> {
        let mut workers_extended: Vec<_> = workers.iter().map(WorkerExtended::from).collect();

        let (worker_from, worker_to) = Self::find_workers(&mut workers_extended, from, to).await?;

        Ok(QueryCoordinator {
            workers: workers_extended,
            query_id,
            node_id_from: from,
            node_id_to: to,
            first_worker_idx: worker_from,
            last_worker_idx: worker_to,
            global_shortest: None,
        })
    }

    async fn find_workers(
        workers: &mut [WorkerExtended],
        from: NodeId,
        to: NodeId,
    ) -> Result<(WorkerIdx, WorkerIdx), Status> {
        let message = generated::worker::NodeIds {
            node_from_id: from,
            node_to_id: to,
        };

        let mut futs = workers
            .iter_mut()
            .enumerate()
            .map(|(idx, worker)| {
                worker
                    .channel
                    .are_nodes_present(Request::new(message.clone()))
                    .map_ok(move |resp| (idx, resp))
            })
            .collect::<FuturesUnordered<_>>();

        let mut from = Option::<WorkerIdx>::default();
        let mut to = Option::<WorkerIdx>::default();

        let assign_ensure_unique = |current: &mut Option<_>, worker_idx| {
            //
            match current.replace(worker_idx) {
                None => Ok(()),
                Some(_) => Err(Status::internal("Two workers cannot contain same node")),
            }
        };

        while let Some((worker_idx, response)) = futs.try_next().await? {
            let result = response.into_inner();

            if result.node_from_present {
                assign_ensure_unique(&mut from, worker_idx)?;
            }

            if result.node_to_present {
                assign_ensure_unique(&mut to, worker_idx)?;
            }

            if from.is_some() && to.is_some() {
                break;
            }
        }

        let from = from.ok_or_else(|| Status::not_found("Requested `from` node not found"))?;
        let to = to.ok_or_else(|| Status::not_found("Requested `to` node not found"))?;

        println!("from_node in worker {}, to_node in worker {}", from, to);
        Ok((from, to))
    }

    fn prepare_outbound_stream(
        &mut self,
        current: WorkerIdx,
    ) -> AsyncStream<RequestDjikstra, impl Future<Output = ()>> {
        let smallest_foreign_node = self.find_shortest_foreign(current);
        let new_nodes = self.workers[current].extract_new_domestic();
        let final_node_id = self.node_id_to;
        let query_id = self.query_id;

        async_stream::stream! {
            yield proto_helpers::pack_query_data(request_djikstra::QueryData {
                query_id,
                final_node_id,
                smallest_foreign_node,
            });

            let node_packed = new_nodes.into_iter().map(proto_helpers::pack_new_domestic_node);

            for node in node_packed {
                yield node;
            }
        }
    }

    pub async fn shortest_path_query(mut self) -> Result<QueryFinished, Status> {
        let mut next_worker = Some(self.first_worker_idx);

        while let Some(current) = next_worker {
            let outbound = self.prepare_outbound_stream(current);
            println!("updating dijkstra for worker {}", current);

            let mut inbound = self.workers[current]
                .channel
                .update_djikstra(outbound)
                .await?
                .into_inner();

            self.workers[current].minimal = None;

            while let Some(response) = inbound.message().await? {
                // FIXME doesn't enter the loop, because inbound.message().await? return None
                println!("updated dijkstra for worker {}", current);

                let message = match response.message_type {
                    Some(msg) => msg,
                    None => {
                        println!("Warning: Empty ResponseDjikstra in the stream");
                        continue;
                    }
                };

                match message {
                    MessageType::Success(s) => {
                        println!("Success: {}", s.shortest_path_len);
                        return Ok(QueryFinished {
                            shortest_path_len: s.shortest_path_len,
                        });
                    }

                    MessageType::NewForeignNode(node) => {
                        println!("NewForeignNode: {}", node.node_id);

                        let worker_idx = self
                            .workers
                            .binary_search_by_key(&node.worker_id, |w| w.id)
                            .map_err(|_| ErrorCollection::worker_not_found(node.worker_id))?;

                        self.workers[worker_idx]
                            .push_new_domestic(node.node_id, node.shortest_path_len);
                        self.update_shortest(worker_idx, node.shortest_path_len);
                    }

                    MessageType::SmallestDomesticNode(node) => {
                        println!("SmallestDomesticNode: {}", node.shortest_path_len);
                        self.workers[current].minimal = Some(node.shortest_path_len);
                    }
                }
            }

            next_worker = self.global_shortest.map(|(worker, _)| worker);
        }

        Err(ErrorCollection::path_not_found())
    }
}

impl ErrorCollection {
    fn worker_not_found(id: WorkerId) -> Status {
        Status::out_of_range(format!("Worker with id: {id} does not exist"))
    }

    fn path_not_found() -> Status {
        Status::internal("Something went wrong and the path was not found")
    }
}

mod proto_helpers {
    use generated::worker::{request_djikstra, RequestDjikstra};

    pub fn pack_query_data(data: request_djikstra::QueryData) -> RequestDjikstra {
        RequestDjikstra {
            message_type: Some(request_djikstra::MessageType::QueryData(data)),
        }
    }

    pub fn pack_new_domestic_node(node: request_djikstra::NewDomesticNode) -> RequestDjikstra {
        RequestDjikstra {
            message_type: Some(request_djikstra::MessageType::NewDomesticNode(node)),
        }
    }
}
