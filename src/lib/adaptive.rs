use std::time::Duration;

use derive_builder::Builder;
use log::trace;

use super::backoff::*;
use super::errors::*;

/// The Adaptive trait defines sucess and fail methods
/// which are used to tune a backoff time.
pub trait Adaptable {
    fn success(&mut self) -> Result<()>;
    fn fail(&mut self) -> Result<()>;
}

impl ExponentialBackoffBuilder {
    /// adaptive finishes the BackoffBuilder and begins an AdaptiveBuilder.
    pub fn adaptive(&mut self) -> Result<AdaptiveBuilder<ExponentialBackoff>> {
        let mut builder = AdaptiveBuilder::default();
        let mut backoff = self.build()?;
        let base_delay = backoff.wait().chain_err(|| "failed to prime base delay")?;
        backoff.reset();

        builder.backoff = Some(backoff);
        builder.base_delay = Some(base_delay);
        Ok(builder)
    }
}

#[derive(Debug, Default, Builder)]
pub struct Adaptive<B: Backoff> {
    backoff: B,
    base_delay: Duration,

    #[builder(default = "0")]
    failures: u32,

    #[builder(default = "0")]
    successes: u32,

    #[builder(setter(skip))]
    #[builder(default = self.base_delay)]
    delay: Duration,
}

impl<B: Backoff> Backoff for Adaptive<B> {
    /// wait returns the current running delay. If success or fail are never called,
    /// it returns zero.
    fn wait(&mut self) -> Result<Duration> {
        Ok(self.delay)
    }

    fn reset(&mut self) {
        self.backoff.reset();
        self.delay = self.base_delay;
        self.successes = 0;
        self.failures = 0;
    }
}

impl<B: Backoff> Adaptable for Adaptive<B> {
    fn success(&mut self) -> Result<()> {
        self.backoff.reset();
        self.successes += 1;
        match self
            .delay
            .checked_sub(self.base_delay.div_f64(self.successes as f64))
        {
            Some(v) => self.delay = v,
            None => self.delay = Duration::new(0, 0),
        }

        trace!(
            "success count now {} with delay @ {:?}",
            self.successes,
            self.delay
        );
        Ok(())
    }

    fn fail(&mut self) -> Result<()> {
        self.failures += 1;
        let delta = self.backoff.wait()?.div_f64(self.failures as f64);
        self.delay += delta;

        trace!(
            "fail count now {}, delta {:?} added to delay, now @ {:?}",
            self.failures,
            delta,
            self.delay
        );

        Ok(())
    }
}

#[test]
fn test_adaptive_exp_backoff() {
    let factor: f64 = 2.0;
    let max_exp = 30;

    let mut backoff = ExponentialBackoffBuilder::default();
    let backoff = backoff
        .factor(2.0)
        .max(factor.powi(max_exp))
        .adaptive()
        .unwrap()
        .build();

    assert!(backoff.is_ok(), backoff.err());
    let mut backoff = backoff.unwrap();

    // all successes, no explicit initial, all zeros
    for i in 0..max_exp {
        let exp = Duration::new(0, 0);
        let res = backoff.success();
        assert!(res.is_ok(), res.err());
        let delay = backoff.wait();

        assert!(delay.is_ok(), delay.err());
        let delay = delay.unwrap();

        assert_eq!(exp, delay, "on iter {}: {:?} != {:?}", i, exp, delay);
    }

    // one failure, then successes carries the failure delay and scales down.
    backoff.reset();
    backoff.fail().unwrap();
    // assume we backed off
    for i in 1..max_exp {
        let res = backoff.success();
        assert!(res.is_ok(), res.err());
        let delay = backoff.wait();

        assert!(delay.is_ok(), delay.err());
        let delay = delay.unwrap();

        // delay is now base + base^1
        let mut exp = factor * 2.0;
        for j in 1..=i {
            exp = (0.0 as f64).max(exp - (factor / j as f64));
        }

        assert!(
            (exp - delay.as_secs_f64()).abs() < 0.01,
            "on iter {}: {:?} != {:?} (within .01)",
            i,
            exp,
            delay
        );
    }
}

/*

/// DelayedAdaptiveBackoff is an AdaptiveBackoff that only begins
/// scaling the backoff (B) after an initial failure. This prevents overtuning
/// as part of a circuit break.
#[derive(Debug, Default)]
pub struct DelayedAdaptiveBackoff<B: Backoff> {
    inner: AdaptiveBackoff<B>,
    triggered: bool,
}

impl<B: Backoff> DelayedAdaptiveBackoff<B> {
    pub fn new() -> Self {
        DelayedAdaptiveBackoff {
            inner: AdaptiveBackoff::<B>::default(),
            triggered: false,
        }
    }

    pub fn with_backoff(inner: B) -> Self {
        DelayedAdaptiveBackoff {
            inner: AdaptiveBackoff::<B>::with_backoff(inner),
            triggered: false,
        }
    }
}

impl<B: Backoff> Backoff for DelayedAdaptiveBackoff<B> {
    /// wait returns the current running delay. If success or fail are never called,
    /// it returns zero.
    fn wait(&mut self) -> Duration {
        self.inner.wait()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}

impl<B: Backoff> Adaptive for DelayedAdaptiveBackoff<B> {
    fn success(&mut self) {
        if self.triggered {
            self.inner.success()
        } else {
            trace!("adaptive success discarded as not triggered");
        }
    }

    fn fail(&mut self) {
        debug!("adaptive backoff triggered");
        self.triggered = true;
        self.inner.fail();
    }
}

*/
