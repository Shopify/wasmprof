use std::{collections::HashMap, os::raw::c_int};

use nix::{libc, sys::signal};
use wasmtime::{Store, WasmBacktrace};
mod collapsed_stack;
mod profile_data;
mod speedscope;
mod timer;

static mut ENGINE: Option<wasmtime::Engine> = None;
static mut TIMER: Option<timer::Timer> = None;
static BACKTRACES: std::sync::Mutex<Vec<(wasmtime::WasmBacktrace, u128)>> =
    std::sync::Mutex::new(vec![]);
static LAST_WEIGHT: std::sync::Mutex<u128> = std::sync::Mutex::new(0);

struct ErrnoProtector(libc::c_int);

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

pub enum WeightUnit {
    Nanoseconds,
    Fuel,
}

pub fn wasmprof<T, FnReturn>(
    frequency: c_int,
    store: &mut wasmtime::Store<T>,
    weight_unit: WeightUnit,
    f: impl FnOnce(&mut Store<T>) -> FnReturn,
) -> (profile_data::ProfileData, FnReturn) {
    register_signal_handler().unwrap();
    unsafe {
        TIMER = Some(timer::Timer::new(frequency));
    }

    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(move |context| {
        if let Some(timer) = unsafe { TIMER.as_ref() } {
            let mut backtraces = BACKTRACES.lock().unwrap();
            let weight = match weight_unit {
                WeightUnit::Nanoseconds => timer.timing().duration.as_nanos(),
                WeightUnit::Fuel => context.fuel_consumed().unwrap_or(0).into(),
            };
            let last_weight = *LAST_WEIGHT.lock().unwrap();
            *LAST_WEIGHT.lock().unwrap() = weight;
            backtraces.push((WasmBacktrace::capture(&context), weight - last_weight));
        }
        Ok(1)
    });

    unsafe { ENGINE = Some(store.engine().clone()) };

    let fn_return = f(store);

    unsafe {
        TIMER = None;
    }
    unregister_signal_handler().unwrap();
    store.epoch_deadline_trap();

    let mut backtraces = BACKTRACES.lock().unwrap();
    let backtraces = std::mem::replace(&mut *backtraces, vec![]);

    let mut name_to_i = HashMap::new();
    let mut frames = Vec::new();
    let mut samples = Vec::new();
    let mut weights = Vec::new();
    for (backtrace, weight) in backtraces {
        let mut sample = Vec::new();
        let bt_frames = backtrace.frames();
        if bt_frames.is_empty() {
            continue;
        }
        for frame in bt_frames {
            let name = frame
                .func_name()
                .map(unmangle_name)
                .unwrap_or_else(|| "<unknown>".to_string());
            let i = *name_to_i.entry(name.to_string()).or_insert_with(|| {
                frames.push(name.to_string());
                frames.len() - 1
            });
            sample.push(i);
        }
        weights.push(weight);
        samples.push(sample);
    }

    unsafe { ENGINE.take() };

    (
        profile_data::ProfileData::new(frames, samples, Some(weights)),
        fn_return,
    )
}

fn unmangle_name(name: &str) -> String {
    if let Ok(demangled) = rustc_demangle::try_demangle(name) {
        demangled.to_string()
    } else if let Ok(demangled) = cpp_demangle::Symbol::new(name) {
        demangled.to_string()
    } else {
        name.to_string()
    }
}
