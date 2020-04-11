use std::time::Duration;

use derive_builder::Builder;

use super::errors::*;

/// The Backoff trait provides a method to return
/// how long to back off when called.
pub trait Backoff {
    fn wait(&mut self) -> Result<Duration>;
    fn reset(&mut self);
}

#[derive(Debug, Default, Builder, Clone)]
pub struct ExponentialBackoff {
    factor: f64,
    initial: Duration,
    max: Duration,
    min: Duration,

    #[builder(setter(skip))]
    #[builder(default = "self.initial")]
    current: Option<Duration>,
}

impl Backoff for ExponentialBackoff {
    fn wait(&mut self) -> Result<Duration> {
        Ok(self.current.unwrap())
    }
    fn reset(&mut self) {
        self.current = Some(self.initial)
    }
}
