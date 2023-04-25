use nix::{libc, sys::signal};
use std::{os::raw::c_int, time::Duration};

use crate::ENGINE;

use super::Error;

mod timer;

struct ErrnoProtector(libc::c_int);

extern "C" fn perf_signal_handler(
    _signal: c_int,
    _siginfo: *mut libc::siginfo_t,
    _ucontext: *mut libc::c_void,
) {
    let _errno = ErrnoProtector::new();

    if let Some(engine) = unsafe { ENGINE.as_ref() } {
        engine.increment_epoch();
    }
}

impl ErrnoProtector {
    fn new() -> Self {
        unsafe {
            #[cfg(target_os = "linux")]
            {
                let errno = *libc::__errno_location();
                Self(errno)
            }
            #[cfg(target_os = "macos")]
            {
                let errno = *libc::__error();
                Self(errno)
            }
        }
    }
}

impl Drop for ErrnoProtector {
    fn drop(&mut self) {
        unsafe {
            #[cfg(target_os = "linux")]
            {
                *libc::__errno_location() = self.0;
            }
            #[cfg(target_os = "macos")]
            {
                *libc::__error() = self.0;
            }
        }
    }
}

fn register_signal_handler() -> nix::Result<()> {
    let handler = signal::SigHandler::SigAction(perf_signal_handler);
    let sigaction = signal::SigAction::new(
        handler,
        // SA_RESTART will only restart a syscall when it's safe to do so,
        // e.g. when it's a blocking read(2) or write(2). See man 7 signal.
        signal::SaFlags::SA_SIGINFO | signal::SaFlags::SA_RESTART,
        signal::SigSet::empty(),
    );
    unsafe { signal::sigaction(signal::SIGPROF, &sigaction) }?;

    Ok(())
}

fn unregister_signal_handler() -> nix::Result<()> {
    let handler = signal::SigHandler::SigIgn;
    unsafe { signal::signal(signal::SIGPROF, handler) }?;

    Ok(())
}

pub struct TickerImpl {
    timer: timer::Timer,
}

impl TickerImpl {
    pub fn new(frequency: u32) -> Result<Self, Error> {
        register_signal_handler().map_err(|_| Error::RegisterError)?;
        Ok(Self {
            timer: timer::Timer::new(frequency),
        })
    }

    pub fn duration(&self) -> Duration {
        self.timer.duration()
    }

    pub fn end(self) -> Result<(), Error> {
        unregister_signal_handler().map_err(|_| Error::UnregisterError)?;
        Ok(())
    }
}
