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
    max: Duration,
    #[builder(default = "Duration::new(0, 0)")]
    min: Duration,

    #[builder(setter(skip))]
    #[builder(default = "1")]
    hits: i32,
}

impl Backoff for ExponentialBackoff {
    fn wait(&mut self) -> Result<Duration> {
        let mut secs = self.factor.powi(self.hits);
        self.hits += 1;
        secs = secs.min(self.min.as_secs_f64());
        secs = secs.max(self.max.as_secs_f64());

        Ok(Duration::from_secs_f64(secs))
    }
    fn reset(&mut self) {
        self.hits = 1;
    }
}
