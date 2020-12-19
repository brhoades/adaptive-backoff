pub(crate) use anyhow::{format_err, Context, Result};
pub(crate) use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdaptiveError {
    #[error("unable to build adaptive backoff: {msg:?}")]
    BuilderFailure { msg: String },
    #[error("missing base delay for adaptive backoff")]
    MissingBaseDelay,
}
