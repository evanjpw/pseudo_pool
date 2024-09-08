mod pool;
mod error;

pub use pool::ExternalPoolEntry as PoolEntry;
pub use pool::Pool;
pub use error::{PseudoPoolError as Error, Result };

