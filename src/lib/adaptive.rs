use std::time::Duration;

use derive_builder::Builder;

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
        self.delay = Duration::new(0, 0);
        self.successes = 0;
        self.failures = 0;
    }
}

impl<B: Backoff> Adaptable for Adaptive<B> {
    fn success(&mut self) -> Result<()> {
        self.successes += 1;
        match self
            .delay
            .checked_sub(self.base_delay.div_f64(self.successes as f64))
        {
            Some(v) => self.delay = v,
            None => self.delay = Duration::new(0, 0),
        }
        Ok(())
    }

    fn fail(&mut self) -> Result<()> {
        self.failures += 1;
        self.delay += self.base_delay.div_f64(self.failures as f64);
        Ok(())
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
