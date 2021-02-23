use std::time::Duration;

#[cfg(test)]
use log::trace;

use crate::{backoff::*, errors::*};

/// The Adaptive trait defines sucess and fail methods
/// which are used to tune a backoff time.
pub trait Adaptable {
    fn success(&mut self) -> Duration;
    fn fail(&mut self) -> Duration;
}

#[derive(Debug, Default)]
pub struct Adaptive<B: Backoff> {
    /// The underlying adaptive backoff, possibly with a carried error.
    backoff: B,

    /// The factor used to increase backoff over time after failed crawls.
    /// On failures, a lower factor leads to smaller delay increases and a higher one
    /// to greater increases.
    /// Must > 0.
    fail_mult: f64,

    /// The factor used to decrease backoff over time after successful crawls.
    /// On success, a lower factor leads to smaller delay decreases and a higher one
    /// to greater decreases.
    /// Must > 0.
    success_mult: f64,

    /// The running factor for failures, available as an initial value.
    /// Factor is used as the running cumulative sum of the backoff.wait() / fail_mult.
    /// It is added to delay on failure().
    fail_factor: f64,

    /// The running factor for successes, available as an initial value.
    /// Factor is used as the running cumulative sum of the base_delay (the first backoff.wait() value)
    /// over success_mult. It is subtracted from delay on success().
    success_factor: f64,

    /// The initial, base delay reset to when reset is called.
    base_delay: Duration,

    /// Running delay value, for free wait calls.
    delay: Duration,

    success_step: f64,
    fail_step: f64,
}

#[derive(Default)]
pub struct AdaptiveBuilder<'a, B: Backoff, BB: BackoffBuilder<B>> {
    builder: Option<&'a mut BB>,
    backoff: Option<B>,

    fail_mult: Option<f64>,
    success_mult: Option<f64>,
    fail_factor: Option<f64>,
    success_factor: Option<f64>,

    initial_delay: Option<Duration>,
}

impl<'a, B: Backoff, BB: BackoffBuilder<B>> AdaptiveBuilder<'a, B, BB> {
    /// sets the underyling backoff function which increases on error.
    /// Cannot be used on an AdaptiveBuilder from a BackoffBuilder.
    pub fn backoff(&mut self, backoff: B) -> &mut Self {
        self.backoff = Some(backoff);
        self
    }

    /// sets the fail multiplier. This multiplier is added to the running
    /// fail factor on each failure.
    pub fn fail_mult(&mut self, m: f64) -> &mut Self {
        self.fail_mult = Some(m);
        self
    }

    /// sets the success multiplier. This multiplier is added to the running
    /// success factor on each success.
    pub fn success_mult(&mut self, m: f64) -> &mut Self {
        self.success_mult = Some(m);
        self
    }

    /// sets the initial fail factor which otherwise defaults to 1.
    /// The fail factor divides the returned backoff amount to decrease
    /// its effects over time.
    pub fn fail_factor(&mut self, f: f64) -> &mut Self {
        self.fail_factor = Some(f);
        self
    }

    /// sets the initial success factor which otherwise defaults to 1.
    /// The success factor divides the base delay value before subtracting
    /// it from the running delay. The running factor decreases the effects
    /// of success over time.
    pub fn success_factor(&mut self, f: f64) -> &mut Self {
        self.success_factor = Some(f);
        self
    }

    /// sets the initial/base delay that the backoff returns to on reset. Defaults to the
    /// backoff's value after a reset.
    pub fn initial_delay(&mut self, d: Duration) -> &mut Self {
        self.initial_delay = Some(d);
        self
    }

    /// sets the initial/base delay that the backoff returns to on reset. Defaults to the
    /// backoff's value after a reset.
    pub fn base_delay(&mut self, d: Duration) -> &mut Self {
        self.initial_delay(d)
    }

    /// build returns the adaptive backoff.
    pub fn build(self) -> Result<Adaptive<B>> {
        let mut backoff = if self.backoff.is_some() && self.builder.is_some() {
            return Err(format_err!("adaptive builders from `.adaptive` on a backoff builder cannot be used with the `.backoff` function"));
        } else if let Some(boff) = self.backoff {
            boff
        } else if let Some(boff) = self.builder {
            boff.build()?
        } else {
            return Err(format_err!("backoff for adaptive backoff builder not specified, must use `.adaptive` on a backoff builder or `.backoff()`"));
        };
        // default to a clean initial backoff
        let base_delay = self.initial_delay.unwrap_or_else(|| {
            let d = backoff.wait();
            backoff.reset();
            d
        });
        let fail_mult = self.fail_mult.unwrap_or(1.0);
        let success_mult = self.success_mult.unwrap_or(1.0);

        Ok(Adaptive::<B> {
            backoff,
            fail_mult,
            success_mult,
            fail_factor: self.fail_factor.unwrap_or_default(),
            success_factor: self.success_factor.unwrap_or_default(),
            base_delay,
            delay: Duration::from_secs_f64(0.0),
            success_step: 1.0 / success_mult,
            fail_step: 1.0 / fail_mult,
        })
    }
}

impl ExponentialBackoffBuilder {
    #[allow(dead_code)]
    pub fn adaptive<'a>(&'a mut self) -> AdaptiveBuilder<'a, ExponentialBackoff, Self> {
        AdaptiveBuilder::<'a, ExponentialBackoff, Self> {
            builder: Some(self),
            ..AdaptiveBuilder::default()
        }
    }
}

impl<B: Backoff> Backoff for Adaptive<B> {
    /// wait returns the current running delay. If success or fail are never called,
    /// it returns zero.
    fn wait(&mut self) -> Duration {
        self.delay
    }

    /// reset returns to the base delay and clears the internal
    /// fail and success factor.
    fn reset(&mut self) {
        self.backoff.reset();
        self.delay = self.base_delay;
        self.success_factor = 0.0;
        self.fail_factor = 0.0;
    }
}

impl<B: Backoff> Adaptable for Adaptive<B> {
    /// success resets the backoff, increases success factor by the success multiplier
    /// and reduces the new returned delay.
    fn success(&mut self) -> Duration {
        self.backoff.reset();
        self.success_factor += self.success_mult;
        match self
            .delay
            .checked_sub(self.base_delay.div_f64(self.success_factor))
        {
            Some(v) => self.delay = v,
            None => self.delay = Duration::new(0, 0),
        }

        #[cfg(test)]
        trace!(
            "success count now {} with delay @ {:?}",
            self.success_factor,
            self.delay
        );
        self.delay
    }

    /// fail uses the backoff and adds it, divided by the fail factor, to
    /// the running delay. It then returns the running delay.
    fn fail(&mut self) -> Duration {
        self.fail_factor += self.fail_mult;
        let delta = self.backoff.wait().div_f64(self.fail_factor);
        self.delay += delta;

        #[cfg(test)]
        trace!(
            "fail count now {}, delta {:?} added to delay, now @ {:?}",
            self.fail_factor,
            delta,
            self.delay
        );
        self.delay
    }
}

#[test]
fn test_adaptive_exp_backoff() {
    let factor: f64 = 2.0;
    let max_exp = 30;

    let mut backoff = ExponentialBackoffBuilder::default()
        .min(Duration::from_secs_f64(0.0))
        .max(Duration::from_secs_f64((2.0 as f64).powi(20)))
        .factor(factor)
        .adaptive()
        .build()
        .unwrap();

    // all success_factor, no explicit initial, all zeros
    for i in 0..max_exp {
        let exp = Duration::new(0, 0);
        let delay = backoff.success();

        assert_eq!(exp, delay, "on iter {}: {:?} != {:?}", i, exp, delay);
    }

    // one failure, then success_factor carries the failure delay and scales down.
    backoff.reset();
    backoff.fail();

    // assume we backed off
    for i in 1..max_exp {
        let delay = backoff.success();

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
