use once_cell::sync::OnceCell;

use crate::graph_store::IdIdxMapping;
use crate::graph_store::SPQGraph;
use crate::QueryProcessorHolder;

// TODO check simpler syntax
pub static GRAPH: OnceCell<SPQGraph> = OnceCell::new();
pub static MAPPING: OnceCell<IdIdxMapping> = OnceCell::new();
pub static PROCESSOR_HOLDER: OnceCell<QueryProcessorHolder> = OnceCell::new();

// Not very pretty but I don't have better idea for that now, maybe macro?
pub fn graph() -> &'static SPQGraph {
    GRAPH.get().unwrap()
}
pub fn mapping() -> &'static IdIdxMapping {
    MAPPING.get().unwrap()
}
pub fn processor_holder() -> &'static QueryProcessorHolder {
    PROCESSOR_HOLDER.get().unwrap()
}
