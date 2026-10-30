use std::time::Duration;

use super::Error;

mod timer;

pub struct TickerImpl {
    timer: timer::Timer,
    signal_id: signal_hook_registry::SigId,
}

impl TickerImpl {
    pub fn new(frequency: u32, engine: wasmtime::Engine) -> Result<Self, Error> {
        let signal_id = unsafe {
            signal_hook_registry::register_sigaction(libc::SIGPROF, move |_| {
                engine.increment_epoch();
            })
        }
        .map_err(|_| Error::RegisterError)?;

        // Timer construction can panic before a Ticker exists for the caller
        // to clean up (for example, when frequency is zero).
        let timer =
            std::panic::catch_unwind(|| timer::Timer::new(frequency)).unwrap_or_else(|panic| {
                signal_hook_registry::unregister(signal_id);
                std::panic::resume_unwind(panic)
            });

        Ok(Self { timer, signal_id })
    }

    pub fn duration(&self) -> Duration {
        self.timer.duration()
    }

    pub fn end(self) -> Result<(), Error> {
        let Self { timer, signal_id } = self;
        drop(timer);

        if !signal_hook_registry::unregister(signal_id) {
            return Err(Error::UnregisterError);
        }

        Ok(())
    }
}
