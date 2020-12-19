use std::time::Duration;

use derive_builder::Builder;
use log::trace;

use crate::{backoff::*, errors::*};

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
        let mut backoff = self
            .build()
            .map_err(|e| format_err!("failed to finish backoff builder: {}", e))?;
        let base_delay = backoff.wait().context("failed to prime base delay")?;
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

    /// The factor used to increase backoff over time after failed crawls.
    /// On failures, a lower factor leads to smaller delay increases and a higher one
    /// to greater increases.
    /// Must > 0.
    #[builder(default = "1.0")]
    fail_mult: f64,

    /// The factor used to decrease backoff over time after successful crawls.
    /// On success, a lower factor leads to smaller delay decreases and a higher one
    /// to greater decreases.
    /// Must > 0.
    #[builder(default = "1.0")]
    success_mult: f64,

    /// The running factor for failures, available as an initial value.
    /// Factor is used as the running cumulative sum of the backoff.wait() / fail_mult.
    /// It is added to delay on failure().
    #[builder(default = "0.0")]
    fail_factor: f64,

    /// The running factor for successes, available as an initial value.
    /// Factor is used as the running cumulative sum of the base_delay (the first backoff.wait() value)
    /// over success_mult. It is subtracted from delay on success().
    #[builder(default = "0.0")]
    success_factor: f64,

    #[builder(setter(skip))]
    #[builder(default = self.base_delay)]
    delay: Duration,

    #[builder(setter(skip))]
    #[builder(field(private))]
    #[builder(default = 1.0 / self.success_mult)]
    success_step: f64,

    #[builder(setter(skip))]
    #[builder(field(private))]
    #[builder(default = 1.0 / self.fail_mult)]
    fail_step: f64,
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
        self.success_factor = 0.0;
        self.fail_factor = 0.0;
    }
}

impl<B: Backoff> Adaptable for Adaptive<B> {
    fn success(&mut self) -> Result<()> {
        self.backoff.reset();
        self.success_factor += self.success_mult;
        match self
            .delay
            .checked_sub(self.base_delay.div_f64(self.success_factor))
        {
            Some(v) => self.delay = v,
            None => self.delay = Duration::new(0, 0),
        }

        trace!(
            "success count now {} with delay @ {:?}",
            self.success_factor,
            self.delay
        );
        Ok(())
    }

    fn fail(&mut self) -> Result<()> {
        self.fail_factor += self.fail_mult;
        let delta = self.backoff.wait()?.div_f64(self.fail_factor);
        self.delay += delta;

        trace!(
            "fail count now {}, delta {:?} added to delay, now @ {:?}",
            self.fail_factor,
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

    // all success_factor, no explicit initial, all zeros
    for i in 0..max_exp {
        let exp = Duration::new(0, 0);
        let res = backoff.success();
        assert!(res.is_ok(), res.err());
        let delay = backoff.wait();

        assert!(delay.is_ok(), delay.err());
        let delay = delay.unwrap();

        assert_eq!(exp, delay, "on iter {}: {:?} != {:?}", i, exp, delay);
    }

    // one failure, then success_factor carries the failure delay and scales down.
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
