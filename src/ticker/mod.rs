#[cfg(all(not(target_os = "windows"), feature = "perf-event"))]
mod signal;
#[cfg(any(target_os = "windows", not(feature = "perf-event")))]
mod thread;

use std::time::Duration;

#[cfg(all(not(target_os = "windows"), feature = "perf-event"))]
pub use signal::TickerImpl;
#[cfg(any(target_os = "windows", not(feature = "perf-event")))]
pub use thread::TickerImpl;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code)]
    RegisterError,
    UnregisterError,
}

pub struct Ticker {
    ticker_impl: TickerImpl,
}

impl Ticker {
    pub fn new(frequency: u32) -> Result<Self, Error> {
        Ok(Self {
            ticker_impl: TickerImpl::new(frequency)?,
        })
    }

    pub fn duration(&self) -> Duration {
        self.ticker_impl.duration()
    }

    pub fn end(self) -> Result<(), Error> {
        self.ticker_impl.end()
    }
}
