#[cfg(all(not(target_os = "windows"), feature = "perf-event"))]
mod signal;
#[cfg(any(target_os = "windows", not(feature = "perf-event")))]
mod thread;

use std::time::{Duration, SystemTime};

#[cfg(all(not(target_os = "windows"), feature = "perf-event"))]
pub use signal::TickerImpl;
#[cfg(any(target_os = "windows", not(feature = "perf-event")))]
pub use thread::TickerImpl;

/// Timing metadata for a collected report.
#[derive(Clone)]
pub struct ReportTiming {
    /// Frequency at which samples were collected.
    pub frequency: i32,
    /// Collection start time.
    pub start_time: SystemTime,
    /// Collection duration.
    pub duration: Duration,
}

impl Default for ReportTiming {
    fn default() -> Self {
        Self {
            frequency: 1,
            start_time: SystemTime::UNIX_EPOCH,
            duration: Default::default(),
        }
    }
}

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
    pub fn new(frequency: i32) -> Result<Self, Error> {
        Ok(Self {
            ticker_impl: TickerImpl::new(frequency)?,
        })
    }

    pub fn timing(&self) -> ReportTiming {
        self.ticker_impl.timing()
    }

    pub fn end(self) -> Result<(), Error> {
        self.ticker_impl.end()
    }
}
