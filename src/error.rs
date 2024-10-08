use crossbeam_channel::{RecvTimeoutError, SendError};
use thiserror::Error;
use uuid::Uuid;

// The Error enum for pseudo_pool. Mostly passthroughs. Thanks `thiserror`!
#[derive(Error, Debug)]
pub enum PseudoPoolError {
    #[error("Invalid checkin of pool entry {0:?}")]
    InvalidCheckin(Uuid),
    #[error(transparent)]
    RecvTimeoutError(#[from] RecvTimeoutError),
    #[error(transparent)]
    SendError(#[from] SendError<()>),
}

pub type Result<T> = std::result::Result<T, PseudoPoolError>;
