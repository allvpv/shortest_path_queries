use crate::executer::{executer_server::Executer, QueryData, QueryFinished};
use crate::workers_connection::{Worker, WorkerId};
use futures::stream::{FuturesUnordered, TryStreamExt};
use futures::TryFutureExt;
use tonic::{Request, Response, Status};

type NodeId = u64;
type WorkersSorted = Vec<Worker>;

pub struct ExecuterService {
    workers: WorkersSorted,
}

impl ExecuterService {
    async fn find_workers(
        workers: &mut WorkersSorted,
        from: NodeId,
        to: NodeId,
    ) -> Result<(WorkerId, WorkerId), Status> {

        let message = crate::worker::NodeIds {
            node_from_id: from,
            node_to_id: to,
        };

        let mut futs = workers
            .iter_mut()
            .map(|worker| {
                worker
                    .channel
                    .are_nodes_present(Request::new(message.clone()))
                    .map_ok(|resp| (worker.id, resp))
            })
            .collect::<FuturesUnordered<_>>();

        let mut from = Option::<WorkerId>::default();
        let mut to = Option::<WorkerId>::default();

        let assign_ensure_unique = |current: &mut Option<_>, worker_id| {
            //
            match current.replace(worker_id) {
                None => Ok(()),
                Some(_) => Err(Status::internal("Two workers cannot contain same node")),
            }
        };

        while let Some((worker_id, response)) = futs.try_next().await? {
            let result = response.into_inner();

            if result.node_from_present {
                assign_ensure_unique(&mut from, worker_id)?;
            }

            if result.node_to_present {
                assign_ensure_unique(&mut to, worker_id)?;
            }

            if from.is_some() && to.is_some() {
                break;
            }
        }

        let from = from.ok_or_else(|| Status::not_found("Requested `from` node not found"))?;
        let to = to.ok_or_else(|| Status::not_found("Requested `to` node not found"))?;

        return Ok((from, to));
    }
}

#[tonic::async_trait]
impl Executer for ExecuterService {
    async fn shortest_path_query(
        &self,
        req: Request<QueryData>,
    ) -> Result<Response<QueryFinished>, Status> {

         // Cloning `Channel` is cheap (and unavoidable, I guess)
        let mut workers = self.workers.clone();

        let QueryData {
            node_id_from,
            node_id_to,
        } = req.into_inner();

        let (first_worker, _) = Self::find_workers(&mut workers, node_id_from, node_id_to).await?;

        unimplemented!()
    }
}
