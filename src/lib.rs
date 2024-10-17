use std::collections::HashMap;
use std::path::PathBuf;

use profile_data::FrameData;
use wasmtime::{Store, UpdateDeadline, WasmBacktrace};
mod collapsed_stack;
mod profile_data;
mod speedscope;
mod ticker;

pub(crate) static mut ENGINE: Option<wasmtime::Engine> = None;
static mut TICKER: Option<ticker::Ticker> = None;
static BACKTRACES: std::sync::Mutex<Vec<(wasmtime::WasmBacktrace, u128)>> =
    std::sync::Mutex::new(vec![]);
static LAST_WEIGHT: std::sync::Mutex<u128> = std::sync::Mutex::new(0);

/// A type to represent the weight unit used by the profiler.
/// The profiler can either use the number of nanoseconds spent in each function
/// or the amount of fuel consumed by each function.
#[derive(Copy, Clone)]
pub enum WeightUnit {
    Nanoseconds,
    Fuel,
}

fn setup_store<T>(store: &mut Store<T>, weight_unit: WeightUnit) {
    store.set_epoch_deadline(1);

    match weight_unit {
        WeightUnit::Fuel => {
            let fuel_max = store
                .get_fuel()
                .expect("Fuel must be set prior to calling setup_store");

            store.epoch_deadline_callback(move |context| {
                let current_fuel = context.get_fuel().expect("Failed to get fuel from context");
                let fuel_consumption = fuel_max.saturating_sub(current_fuel);
                let weight = fuel_consumption.into();

                add_weighted_backtrace(context, weight);

                Ok(UpdateDeadline::Continue(1))
            });
        }
        WeightUnit::Nanoseconds => {
            store.epoch_deadline_callback(move |context| {
                if let Some(ticker) = unsafe { TICKER.as_ref() } {
                    let weight = ticker.duration().as_nanos();

                    add_weighted_backtrace(context, weight);
                }
                Ok(UpdateDeadline::Continue(1))
            });
        }
    }
}

fn add_weighted_backtrace<T>(context: wasmtime::StoreContextMut<'_, T>, weight: u128) {
    let mut backtraces = BACKTRACES.lock().unwrap();

    let last_weight = *LAST_WEIGHT.lock().unwrap();
    *LAST_WEIGHT.lock().unwrap() = weight;
    backtraces.push((WasmBacktrace::capture(context), weight - last_weight));
}

/// A builder for the profiler. It allows to set the frequency at which the profiler
/// will sample the stack and the weight unit used by the profiler.
/// The profiler will start when the `profile` method is called.
pub struct ProfilerBuilder<'a, T> {
    frequency: u32,
    weight_unit: WeightUnit,
    store: &'a mut wasmtime::Store<T>,
    binary_path: Option<PathBuf>,
}

impl<'a, T> ProfilerBuilder<'a, T> {
    pub fn new(store: &'a mut wasmtime::Store<T>) -> Self {
        Self {
            frequency: 1000,
            weight_unit: WeightUnit::Nanoseconds,
            store,
            binary_path: None,
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

    pub fn binary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.binary_path = Some(path.into());
        self
    }

    /// Start the profiler and call the function `f` with the store.
    /// It returns the value returned by `f` and the data collected by the profiler.
    pub fn profile<FnReturn>(
        self,
        f: impl FnOnce(&mut Store<T>) -> FnReturn,
    ) -> (FnReturn, profile_data::ProfileData) {
        let ticker = ticker::Ticker::new(self.frequency).unwrap();
        unsafe {
            TICKER = Some(ticker);
        }
        unsafe { ENGINE = Some(self.store.engine().clone()) };

        setup_store(self.store, self.weight_unit);

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
                let frame_data = FrameData::from((frame, self.binary_path.as_ref()));
                let i = *name_to_i.entry(frame_data.name.clone()).or_insert_with(|| {
                    frames.push(frame_data);
                    frames.len() - 1
                });
                sample.push(i);
            }
            weights.push(weight);
            samples.push(sample);
        }

        let profile_data = profile_data::ProfileData::new(frames, samples, Some(weights), self.weight_unit, self.binary_path.clone());

        unsafe { ENGINE.take() };

        (fn_return, profile_data)
    }
}

pub use profile_data::ProfileData;
pub use speedscope::SpeedscopeFile;
