use std::collections::HashMap;

use wasmtime::{Store, WasmBacktrace, AsContextMut};
mod collapsed_stack;
mod profile_data;
mod ticker;

pub(crate) static mut ENGINE: Option<wasmtime::Engine> = None;
static mut TICKER: Option<ticker::Ticker> = None;
static BACKTRACES: std::sync::Mutex<Vec<(wasmtime::WasmBacktrace, u128)>> =
    std::sync::Mutex::new(vec![]);
static LAST_WEIGHT: std::sync::Mutex<u128> = std::sync::Mutex::new(0);

#[derive(Clone, Copy)]
/// A type to represent the weight unit used by the profiler.
/// The profiler can either use the number of nanoseconds spent in each function
/// or the amount of fuel consumed by each function.
pub enum WeightUnit {
    Nanoseconds,
    Fuel,
}

/// A builder for the profiler. It allows to set the frequency at which the profiler
/// will sample the stack and the weight unit used by the profiler.
/// The profiler will start when the `profile` method is called.
pub struct ProfilerBuilder<'a, T> {
    frequency: u32,
    weight_unit: WeightUnit,
    store: &'a mut wasmtime::Store<T>,
    instance: Option<wasmtime::Instance>,
    runtimes: Vec<String>,
}

fn read_runtimes_stack_traces(mut context: impl AsContextMut, instance: &mut Option<wasmtime::Instance>, runtimes: &[String]) {
    if let Some(instance) = instance {
        for runtime in runtimes {
            let stack_getter = instance.get_typed_func::<(), i32>(context.as_context_mut(), format!("__wasmprof_stacks_{}", runtime).as_str());
            if let Ok(stack_getter) = stack_getter {
                let stack = stack_getter.call(context.as_context_mut(), ());
                match stack {
                    Ok(stack) => {
                        println!("{}: {:?}", runtime, stack);
                    },
                    Err(_) => {
                        println!("{}: no stack", runtime);
                    }
                }
                println!("{}: {:?}", runtime, stack);
            }
        }
    }
}

impl<'a, T> ProfilerBuilder<'a, T> {
    pub fn new(store: &'a mut wasmtime::Store<T>) -> Self {
        Self {
            frequency: 1000,
            weight_unit: WeightUnit::Nanoseconds,
            store,
            instance: None,
            runtimes: vec![],
        }
    }

    /// sets the frequency in Hz at which the profiler will sample the stack.
    pub fn frequency(mut self, frequency: u32) -> Self {
        self.frequency = frequency;
        self
    }

    pub fn weight_unit(mut self, weight_unit: WeightUnit) -> Self {
        self.weight_unit = weight_unit;
        self
    }

    /// sets the instance to profile. this is only useful if you want to profile
    /// code running in an language runtime like js or ruby.
    pub fn instance(mut self, instance: wasmtime::Instance) -> Self {
        self.instance = Some(instance);
        self
    }

    /// sets the runtimes to profile. this is only useful if you want to profile
    /// code running in an language runtime like js or ruby.
    pub fn add_runtimes(mut self, runtime: String) -> Self {
        self.runtimes.push(runtime);
        self
    }

    /// Start the profiler and call the function `f` with the store.
    /// It returns the value returned by `f` and the data collected by the profiler.
    pub fn profile<FnReturn>(
        mut self,
        f: impl FnOnce(&mut Store<T>) -> FnReturn,
    ) -> (FnReturn, profile_data::ProfileData) {
        let ticker = ticker::Ticker::new(self.frequency).unwrap();
        unsafe {
            TICKER = Some(ticker);
        }
        unsafe { ENGINE = Some(self.store.engine().clone()) };

        self.setup_store();

        let fn_return = f(self.store);

        let ticker = unsafe { TICKER.take() };
        if let Some(ticker) = ticker {
            ticker.end().unwrap();
        }

        self.store.epoch_deadline_trap();

        let mut backtraces = BACKTRACES.lock().unwrap();
        let backtraces = std::mem::take(&mut *backtraces);

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
            fn_return,
            profile_data::ProfileData::new(frames, samples, Some(weights)),
        )
    }

    fn setup_store(&mut self) {
        let mut instance = self.instance.take();
        let runtimes = std::mem::take(&mut self.runtimes);
        self.store.set_epoch_deadline(1);
        let weight_unit = self.weight_unit;
        self.store.epoch_deadline_callback(move |mut context| {
            if let Some(ticker) = unsafe { TICKER.as_ref() } {
                let mut backtraces = BACKTRACES.lock().unwrap();
                let weight = match weight_unit {
                    WeightUnit::Nanoseconds => ticker.duration().as_nanos(),
                    WeightUnit::Fuel => context.fuel_consumed().unwrap_or(0).into(),
                };
                let last_weight = *LAST_WEIGHT.lock().unwrap();
                *LAST_WEIGHT.lock().unwrap() = weight;
                backtraces.push((WasmBacktrace::capture(&context), weight - last_weight));
            }
            read_runtimes_stack_traces(context.as_context_mut(), &mut instance, &runtimes);
            Ok(1)
        });
    }
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
