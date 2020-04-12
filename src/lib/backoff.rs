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
    max: f64,
    #[builder(default = "0.0")]
    min: f64,

    #[builder(setter(skip))]
    #[builder(default = "1")]
    hits: i32,
}

impl Backoff for ExponentialBackoff {
    fn wait(&mut self) -> Result<Duration> {
        let mut secs = self.factor.powi(self.hits);
        self.hits += 1;
        secs = secs.max(self.min).max(self.min);

        Ok(Duration::from_secs_f64(secs))
    }
    fn reset(&mut self) {
        self.hits = 1;
    }
}

#[test]
fn test_exp_backoff() {
    let backoff = ExponentialBackoffBuilder::default()
        .min(0.0)
        .max((2.0 as f64).powi(20))
        .factor(2.0)
        .build();

    assert!(backoff.is_ok(), backoff.err());
    let mut backoff = backoff.unwrap();

    for i in 1..20 {
        let delay = backoff.wait();
        let ex = Duration::new((2 as u64).pow(i), 0);
        assert!(delay.is_ok(), delay.err());

        let delay = delay.unwrap();
        assert!(ex == delay, "on iter {}: {:?} != {:?}", i, ex, delay);
    }

    backoff.reset();

    let delay = backoff.wait();
    assert!(delay.is_ok(), delay.err());
    let delay = delay.unwrap();
    assert!(
        delay == Duration::new(2, 0),
        "after reset: {:?} != {:?}",
        Duration::new(2, 0),
        delay
    );
}
