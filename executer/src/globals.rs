use once_cell::sync::OnceCell;

use crate::queries_manager::QueriesManager;

pub static QUERIES_MANAGER: OnceCell<QueriesManager> = OnceCell::new();

// Not very pretty but I don't have better idea for that now, maybe macro?
pub fn queries_manager() -> &'static QueriesManager {
    QUERIES_MANAGER.get().unwrap()
}
