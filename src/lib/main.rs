use std::time::Duration;

use log::{debug, trace};

pub mod prelude {
    pub use super::{Adaptive, Backoff};
    pub use super::{AdaptiveBackoff, DelayedAdaptiveBackoff};
}

/// The Backoff trait provides a method to return
/// how long to back off when called.
pub trait Backoff: Default {
    fn wait(&mut self) -> std::time::Duration;
    fn reset(&mut self);
}

/// The Adaptive trait defines sucess and fail methods
/// which are used to tune a backoff time.
pub trait Adaptive: Default {
    fn success(&mut self);
    fn fail(&mut self);
}

#[derive(Debug, Default)]
pub struct AdaptiveBackoff<B: Backoff> {
    inner: B,
    hits: u32,
    delay: std::time::Duration,
}

impl<B: Backoff> AdaptiveBackoff<B> {
    pub fn new() -> Self {
        Self::with_backoff(B::default())
    }

    pub fn with_backoff(inner: B) -> Self {
        let mut adapt = Self::new();

        adapt.inner = inner;
        adapt
    }
}

impl<B: Backoff> Backoff for AdaptiveBackoff<B> {
    /// wait returns the current running delay. If success or fail are never called,
    /// it returns zero.
    fn wait(&mut self) -> Duration {
        self.delay
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.delay = Duration::new(0, 0);
    }
}

impl<B: Backoff> Adaptive for AdaptiveBackoff<B> {
    fn success(&mut self) {
        self.hits += 1;
        self.inner.reset();
    }

    fn fail(&mut self) {
        self.hits += 1;
        self.delay += Duration::from_secs_f64(self.inner.wait().as_secs_f64() / self.hits as f64);
    }
}

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
