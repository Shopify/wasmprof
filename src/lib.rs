use std::collections::HashMap;

use wasmtime::{AsContextMut, Store, WasmBacktrace};
mod collapsed_stack;
mod profile_data;
mod ticker;

pub(crate) static mut ENGINE: Option<wasmtime::Engine> = None;
static mut TICKER: Option<ticker::Ticker> = None;
static BACKTRACES: std::sync::Mutex<Option<HashMap<String, Vec<Sample>>>> =
    std::sync::Mutex::new(None);
static LAST_WEIGHT: std::sync::Mutex<u128> = std::sync::Mutex::new(0);

#[derive(Clone, Copy)]
/// A type to represent the weight unit used by the profiler.
/// The profiler can either use the number of nanoseconds spent in each function
/// or the amount of fuel consumed by each function.
pub enum WeightUnit {
    Nanoseconds,
    Fuel,
}

pub enum Backtrace {
    Native(WasmBacktrace),
    Runtime(Vec<String>),
}

impl Backtrace {
    pub fn from_wasm_backtrace(backtrace: WasmBacktrace) -> Self {
        Backtrace::Native(backtrace)
    }

    pub fn from_runtime_backtrace(backtrace: Vec<String>) -> Self {
        Backtrace::Runtime(backtrace)
    }

    pub fn frames(&self) -> Vec<String> {
        match self {
            Backtrace::Native(backtrace) => backtrace.frames().iter().map(|frame| {
                frame
                    .func_name()
                    .map(unmangle_name)
                    .unwrap_or_else(|| "<unknown>".to_string())
            }).collect(),
            Backtrace::Runtime(backtrace) => backtrace.clone(),
        }
    }
}

pub struct Sample {
    backtrace: Backtrace,
    weight: u128,
}

impl Sample {
    fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    fn weight(&self) -> u128 {
        self.weight
    }
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

fn read_runtimes_stack_traces(
    mut context: impl AsContextMut,
    instance: &mut Option<wasmtime::Instance>,
    runtimes: &[String],
    backtraces: &mut HashMap<String, Vec<Sample>>,
    weight: u128,
) {
    if let Some(instance) = instance {
        for runtime in runtimes {
            let stack_creator = instance.get_typed_func::<(), i32>(
                context.as_context_mut(),
                format!("__{}_wasmprof_stacks_create", runtime).as_str(),
            );
            let stack_ptr_getter = instance.get_typed_func::<i32, i32>(
                context.as_context_mut(),
                format!("__{}_wasmprof_stacks_get", runtime).as_str(),
            );
            let stack_len_getter = instance.get_typed_func::<i32, i32>(
                context.as_context_mut(),
                format!("__{}_wasmprof_stacks_len", runtime).as_str(),
            );
            let stack_destroyer = instance.get_typed_func::<i32, ()>(
                context.as_context_mut(),
                format!("__{}_wasmprof_stacks_destroy", runtime).as_str(),
            );

            match (
                stack_creator,
                stack_ptr_getter,
                stack_len_getter,
                stack_destroyer,
            ) {
                (
                    Ok(stack_creator),
                    Ok(stack_ptr_getter),
                    Ok(stack_len_getter),
                    Ok(stack_destroyer),
                ) => {
                    let stack = stack_creator.call(context.as_context_mut(), ()).unwrap();
                    let ptr = stack_ptr_getter
                        .call(context.as_context_mut(), stack)
                        .unwrap();
                    let len = stack_len_getter
                        .call(context.as_context_mut(), stack)
                        .unwrap();
                    let mem = instance
                        .get_memory(context.as_context_mut(), "memory")
                        .unwrap();
                    let mut buf = vec![0u8; len as usize];
                    mem.read(context.as_context_mut(), ptr as usize, &mut buf[..])
                        .unwrap();
                    let read: Vec<String> = rmp_serde::from_slice(&buf).unwrap();

                    let backtrace = backtraces
                        .entry(runtime.to_string())
                        .or_insert_with(|| vec![]);
                    backtrace.push(Sample {
                        backtrace: Backtrace::from_runtime_backtrace(read),
                        weight,
                    });

                    stack_destroyer
                        .call(context.as_context_mut(), stack)
                        .unwrap();
                }
                _ => {
                    println!("{}: no stack", runtime);
                }
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
        *(BACKTRACES.lock().unwrap()) =
            Some(HashMap::from_iter([("native".to_string(), Vec::new())]));

        self.setup_store();

        let fn_return = f(self.store);

        let ticker = unsafe { TICKER.take() };
        if let Some(ticker) = ticker {
            ticker.end().unwrap();
        }

        self.store.epoch_deadline_trap();

        let mut backtraces = BACKTRACES.lock().unwrap();
        let backtraces = backtraces.as_mut().unwrap();
        let backtraces = std::mem::take(backtraces);
        let native_backtraces = backtraces.get("native").unwrap();

        let mut name_to_i = HashMap::new();
        let mut frames = Vec::new();
        let mut samples = Vec::new();
        let mut weights = Vec::new();
        for sample in native_backtraces {
            let backtrace = sample.backtrace();
            let weight = sample.weight();
            let mut sample = Vec::new();
            let bt_frames = backtrace.frames();
            if bt_frames.is_empty() {
                continue;
            }
            for name in bt_frames {
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
                let backtraces = backtraces.as_mut().unwrap();
                let weight = match weight_unit {
                    WeightUnit::Nanoseconds => ticker.duration().as_nanos(),
                    WeightUnit::Fuel => context.fuel_consumed().unwrap_or(0).into(),
                };
                let last_weight = *LAST_WEIGHT.lock().unwrap();
                *LAST_WEIGHT.lock().unwrap() = weight;
                let weight = weight - last_weight;
                let backtrace = Backtrace::Native(WasmBacktrace::capture(&context));
                backtraces
                    .get_mut("native")
                    .unwrap()
                    .push(Sample { backtrace, weight });
                context.set_epoch_deadline(100000);
                // read_runtimes_stack_traces(
                //     context.as_context_mut(),
                //     &mut instance,
                //     &runtimes,
                //     backtraces,
                //     weight,
                // );
                context.set_epoch_deadline(0);
            }

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
