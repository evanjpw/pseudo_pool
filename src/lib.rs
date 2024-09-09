mod error;
mod pool;

pub use error::{PseudoPoolError as Error, Result};
pub use pool::ExternalPoolEntry as PoolEntry;
pub use pool::Pool;
