use std::time::Duration;

use super::errors::*;

/// The Backoff trait provides a method to return
/// how long to back off when called.
pub trait Backoff {
    /// wait for the next backoff duration. Each backoff
    /// function will vary in how the duration changes with time.
    fn wait(&mut self) -> Duration;

    /// reset the backoff duration to the initial if it has some
    /// running state.
    fn reset(&mut self);
}

#[derive(Debug, Default, Clone)]
pub struct ExponentialBackoff {
    factor: f64,
    max: Option<f64>,
    min: f64,

    hits: i32,
}

impl Backoff for ExponentialBackoff {
    fn wait(&mut self) -> Duration {
        let mut secs = self.factor.powi(self.hits);
        self.hits += 1;
        secs = secs.max(self.min);
        if let Some(max) = self.max {
            secs = secs.min(max)
        }

        Duration::from_secs_f64(secs)
    }

    fn reset(&mut self) {
        self.hits = 1;
    }
}

pub trait BackoffBuilder<T: Backoff> {
    fn build(&mut self) -> Result<T>;
}

#[derive(Default, Debug)]
pub struct ExponentialBackoffBuilder {
    min: Option<Duration>,
    max: Option<Duration>,
    factor: Option<f64>,
}

impl BackoffBuilder<ExponentialBackoff> for ExponentialBackoffBuilder {
    /// Build finishes the exponential backoff and returns it or an error.
    fn build(&mut self) -> Result<ExponentialBackoff> {
        Ok(ExponentialBackoff {
            min: self.min.ok_or_else(|| format_err!("the minimum initial value is required"))?.as_secs_f64(),
            max: self.max.map(|s| s.as_secs_f64()),
            hits: 1,
            factor: self.factor.unwrap_or(2.0),
            ..ExponentialBackoff::default()
        })
    }
}

impl ExponentialBackoffBuilder {
    /// The minimum and initial delay of the backoff.
    pub fn min(&mut self, min: Duration) -> &mut Self {
        self.min = Some(min);
        self
    }

    /// The capped maximum delay that can be returned.
    pub fn max(&mut self, max: Duration) -> &mut Self {
        self.max = Some(max);
        self
    }

    /// The factor that the delay increases by on each hit. Defaults to 2.0.
    pub fn factor(&mut self, f: f64) -> &mut Self {
        self.factor = Some(f);
        self
    }
}

#[test]
fn test_exp_backoff() {
    let backoff = ExponentialBackoffBuilder::default()
        .min(Duration::from_secs_f64(0.0))
        .max(Duration::from_secs_f64((2.0 as f64).powi(20)))
        .factor(2.0)
        .build();

    assert!(backoff.is_ok(), backoff.err());
    let mut backoff = backoff.unwrap();

    for i in 1..20 {
        let delay = backoff.wait();
        let ex = Duration::from_secs_f64((2.0 as f64).powi(i));

        assert!(ex == delay, "on iter {}: {:?} != {:?}", i, ex, delay);
    }

    backoff.reset();

    let delay = backoff.wait();
    assert!(
        delay == Duration::new(2, 0),
        "after reset: {:?} != {:?}",
        Duration::new(2, 0),
        delay
    );
}
