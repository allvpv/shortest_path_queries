use async_stream::AsyncStream;
use futures::future::try_join_all;
use futures::stream::FuturesUnordered;
use futures::Future;
use futures::TryFutureExt;
use futures::TryStreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tonic::Result;
use tonic::Status;

use generated::executer;
use generated::worker;

use executer::QueryResults;
use worker::worker_client::WorkerClient;
use worker::{request_djikstra, response_djikstra};
use worker::{ForgetQueryMessage, RequestDjikstra};

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
    minimal: Option<ShortestPathLen>,
    new_nodes: Vec<NewDomesticNode>,
    is_involved: bool, // Was the worker involved in the current query?
}

impl WorkerExtended {
    fn from(worker: &Worker) -> Self {
        WorkerExtended {
            id: worker.id,
            channel: worker.channel.clone(), // Cloning `Channel` is cheap (and unavoidable I guess)
            minimal: None,
            new_nodes: Vec::new(),
            is_involved: false,
        }
    }

    fn push_new_domestic(
        &mut self,
        node_id: NodeId,
        shortest_path_len: ShortestPathLen,
        parent_node: Option<(NodeId, WorkerId)>,
    ) {
        let parent_node =
            parent_node.map(|(node_id, worker_id)| worker::NodePointer { worker_id, node_id });

        self.new_nodes.push(NewDomesticNode {
            node_id,
            shortest_path_len,
            parent_node,
        });

        let update = self.minimal.is_none() || self.minimal.unwrap() > shortest_path_len;

        if update {
            self.minimal = Some(shortest_path_len);
        };
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
}

impl QueryCoordinator {
    fn find_shortest_foreign(&self, current: WorkerIdx) -> Option<ShortestPathLen> {
        self.workers
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != current)
            .filter_map(|(_, w)| w.minimal)
            .min()
    }

    pub async fn send_forget_to_workers(&mut self) -> Result<(), Status> {
        let query_id = self.query_id;
        let futures = self
            .workers
            .iter_mut()
            .filter(|worker| worker.is_involved)
            .map(|worker| {
                info!(" -> sending forget request to worker[id: {}]", worker.id);
                worker.channel.forget_query(ForgetQueryMessage { query_id })
            })
            .collect::<Vec<_>>();

        try_join_all(futures).await?;

        Ok(())
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
                Some(_) => Err(Status::internal("two workers cannot contain same node")),
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

        let from = from.ok_or_else(|| Status::not_found("requested `from` node not found"))?;
        let to = to.ok_or_else(|| Status::not_found("requested `to` node not found"))?;

        debug!("node `from` found in worker[id {from}]");
        debug!("node `to` found in worker[id {to}]");

        Ok((from, to))
    }

    fn prepare_outbound_stream(
        &mut self,
        current: WorkerIdx,
    ) -> AsyncStream<RequestDjikstra, impl Future<Output = ()>> {
        let query_data = request_djikstra::QueryData {
            query_id: self.query_id,
            final_node_id: self.node_id_to,
            smallest_foreign_node: self.find_shortest_foreign(current),
        };

        debug!("sending `update_dijkstra` request to worker[idx {current}]");
        debug!(" -> query_data: {query_data:?}");

        let new_nodes = self.workers[current].extract_new_domestic();

        async_stream::stream! {
            yield proto_helpers::pack_query_data(query_data);

            for node in new_nodes.into_iter() {
                debug!(" -> node: {node:?}");
                yield proto_helpers::pack_new_domestic_node(node);
            }
        }
    }

    pub async fn shortest_path_query(&mut self) -> Result<QueryResults, Status> {
        // Push initial node
        self.workers[self.first_worker_idx].push_new_domestic(self.node_id_from, 0, None);

        let mut next_worker = Some(self.first_worker_idx);

        while let Some(current) = next_worker {
            debug!("current worker: {}", current);

            let outbound = self.prepare_outbound_stream(current);

            let mut inbound = self.workers[current]
                .channel
                .update_djikstra(outbound)
                .await?
                .into_inner();

            debug!("parsing `update_dijkstra` response from worker[idx {current}]:");

            self.workers[current].minimal = None;
            self.workers[current].is_involved = true;
            let current_worker_id = self.workers[current].id;

            while let Some(response) = inbound.message().await? {
                let message = match response.message_type {
                    Some(msg) => msg,
                    None => {
                        warn!(" -> empty `ResponseDjikstra` in the stream!");
                        continue;
                    }
                };

                match message {
                    MessageType::Success(s) => {
                        debug!(" -> query finished with success: {}", s.shortest_path_len);

                        return Ok(QueryResults {
                            shortest_path_len: Some(s.shortest_path_len),
                            query_id: Some(self.query_id),
                        });
                    }

                    MessageType::NewForeignNode(node) => {
                        debug!(" -> received foreign node {node:?}");

                        let this_node = node.this_node.ok_or_else(|| {
                            Status::invalid_argument(
                                "Empty `this_node` in `NewForeignNode` message",
                            )
                        })?;

                        let worker_idx = self
                            .workers
                            .binary_search_by_key(&this_node.worker_id, |w| w.id)
                            .map_err(|_| ErrorCollection::worker_not_found(this_node.worker_id))?;

                        debug!(
                            " -> node[id {}] belongs to worker[idx {}]",
                            this_node.node_id, worker_idx
                        );

                        self.workers[worker_idx].push_new_domestic(
                            this_node.node_id,
                            node.shortest_path_len,
                            Some((node.parent_node_id, current_worker_id)),
                        );
                    }

                    MessageType::SmallestDomesticNode(node) => {
                        debug!(
                            " -> smallest domestic node has len: {}",
                            node.shortest_path_len
                        );

                        self.workers[current].minimal = Some(node.shortest_path_len);
                    }
                }
            }

            debug!("finished parsing `update_djikstra` response from worker");

            next_worker = self
                .workers
                .iter()
                .enumerate()
                .filter(|(_, w)| w.minimal.is_some())
                .min_by_key(|(_, w)| w.minimal)
                .map(|(idx, _)| idx);
        }

        debug!("path was not found");

        // Path not found
        return Ok(QueryResults {
            shortest_path_len: None,
            query_id: Some(self.query_id),
        });
    }
}

impl ErrorCollection {
    fn worker_not_found(id: WorkerId) -> Status {
        Status::out_of_range(format!("worker[id: {id}] does not exist"))
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
