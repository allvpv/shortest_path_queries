use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use tonic::Status;

use generated::worker::request_djikstra::QueryData;

use crate::graph_store::{IdIdxMapping, SPQGraph};
use crate::query_processor::{QueryId, QueryProcessor};
use crate::ErrorCollection;

#[derive(Debug)]
pub struct QueryProcessorHolder {
    graph: Arc<SPQGraph>,
    mapping: Arc<IdIdxMapping>,
    processors_map: Mutex<QueryProcessorMap>,
}

#[derive(Debug)]
enum QueryProcessorEntry {
    Busy, // The query is pending
    Ready(QueryProcessor),
}

type QueryProcessorMap = HashMap<QueryId, QueryProcessorEntry>;

impl QueryProcessorHolder {
    pub fn new(graph: SPQGraph, mapping: IdIdxMapping) -> Self {
        QueryProcessorHolder {
            graph: Arc::new(graph),
            mapping: Arc::new(mapping),
            processors_map: Mutex::new(QueryProcessorMap::new()),
        }
    }

    pub fn get_mapping(&self) -> &IdIdxMapping {
        &self.mapping
    }

    // Gets QueryProcessor for the query. If this is the first request for this query, creates new.
    pub fn get_for_query(&self, query_data: &QueryData) -> Result<QueryProcessor, Status> {
        let processor = match self.get_processor(query_data.query_id)? {
            Some(processor) => processor,
            None => {
                let graph = Arc::clone(&self.graph);
                let mapping = Arc::clone(&self.mapping);
                QueryProcessor::new(graph, mapping, query_data)
            }
        };

        Ok(processor)
    }

    pub fn put_back_query(&self, processor: QueryProcessor) -> Result<(), Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let mut processor_map = self
            .processors_map
            .lock()
            .map_err(ErrorCollection::locking_mutex)?;

        let mut entry = match processor_map.entry(processor.query_id()) {
            Occupied(entry) => entry,
            Vacant(_) => unreachable!(),
        };

        use QueryProcessorEntry::{Busy, Ready};

        let previous_value = entry.insert(Ready(processor));
        debug_assert!(matches!(previous_value, Busy));

        Ok(())
    }

    // Drops the query processor, and thus all information about this query
    pub fn forget_query(&self, processor: QueryProcessor) -> Result<(), Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let mut processor_map = self
            .processors_map
            .lock()
            .map_err(ErrorCollection::locking_mutex)?;

        let entry = match processor_map.entry(processor.query_id()) {
            Occupied(entry) => entry,
            Vacant(_) => unreachable!(),
        };

        let previous_value = entry.remove();
        debug_assert!(matches!(previous_value, QueryProcessorEntry::Busy));

        Ok(())
    }

    pub fn get_processor(&self, query_id: QueryId) -> Result<Option<QueryProcessor>, Status> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        use QueryProcessorEntry::{Busy, Ready};

        let mut queries = self
            .processors_map
            .lock()
            .map_err(ErrorCollection::locking_mutex)?;

        let processor = match queries.entry(query_id) {
            // No QueryProcessor for this query_id was created, but we will create one soon, so
            // insert Busy into the holder.
            Vacant(entry) => {
                entry.insert(Busy);
                None
            }
            Occupied(mut entry) => {
                // QueryProcessor for this query_id was already created.
                match entry.insert(Busy) {
                    Ready(processor) => Some(processor),
                    // If it is busy, than there is an error in Executer; new request for this
                    // query came before previous was finished.
                    Busy => return Err(ErrorCollection::processor_busy()),
                }
            }
        };

        Ok(processor)
    }
}

impl ErrorCollection {
    fn processor_busy() -> Status {
        Status::invalid_argument("Cannot get busy QueryProcessor")
    }

    fn locking_mutex(e: impl std::error::Error) -> Status {
        Status::internal(format!("Internal error while locking mutex: {e}"))
    }
}
