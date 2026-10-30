use std::{collections::HashMap, sync::Mutex};

use wasmtime::{Store, UpdateDeadline, WasmBacktrace};
mod collapsed_stack;
mod profile_data;
mod ticker;

static mut TICKER: Option<ticker::Ticker> = None;
static ACTIVE_PROFILE: Mutex<()> = Mutex::new(());
static BACKTRACES: Mutex<Vec<(wasmtime::WasmBacktrace, u128)>> = Mutex::new(vec![]);
static LAST_WEIGHT: Mutex<u128> = Mutex::new(0);

/// A type to represent the weight unit used by the profiler.
/// The profiler can either use the number of nanoseconds spent in each function
/// or the amount of fuel consumed by each function.
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
                #[allow(static_mut_refs)]
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
    backtraces.push((WasmBacktrace::force_capture(context), weight - last_weight));
}

/// A builder for the profiler. It allows to set the frequency at which the profiler
/// will sample the stack and the weight unit used by the profiler.
/// The profiler will start when the `profile` method is called.
pub struct ProfilerBuilder<'a, T: 'static> {
    frequency: u32,
    weight_unit: WeightUnit,
    store: &'a mut wasmtime::Store<T>,
}

impl<'a, T> ProfilerBuilder<'a, T> {
    pub fn new(store: &'a mut wasmtime::Store<T>) -> Self {
        Self {
            frequency: 1000,
            weight_unit: WeightUnit::Nanoseconds,
            store,
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

    /// Start the profiler and call the function `f` with the store.
    /// It returns the value returned by `f` and the data collected by the profiler.
    pub fn profile<FnReturn>(
        self,
        f: impl FnOnce(&mut Store<T>) -> FnReturn,
    ) -> (FnReturn, profile_data::ProfileData) {
        let _active_profile = ACTIVE_PROFILE
            .try_lock()
            .expect("only one profiler can be active at a time");
        // Elapsed time and fuel consumption restart at zero for each session.
        *LAST_WEIGHT.lock().unwrap() = 0;
        let ticker = ticker::Ticker::new(self.frequency, self.store.engine().clone()).unwrap();
        unsafe {
            TICKER = Some(ticker);
        }

        setup_store(self.store, self.weight_unit);

        let fn_return = f(self.store);

        #[allow(static_mut_refs)]
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

        (
            fn_return,
            profile_data::ProfileData::new(frames, samples, Some(weights)),
        )
    }
}

fn unmangle_name(name: &str) -> String {
    if let Ok(demangled) = rustc_demangle::try_demangle(name) {
        demangled.to_string()
    } else if let Ok(cpp_symbol) = cpp_demangle::Symbol::new(name) {
        cpp_symbol.demangle().unwrap_or_else(|_| name.to_string())
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, thread, time::Duration};

    use wasmtime::{Config, Engine, Instance, Module, Store};

    use super::{unmangle_name, ProfilerBuilder, LAST_WEIGHT};

    // Tests that start profiles must not overlap with each other.
    static PROFILE_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn unmangles_rust_symbols() {
        assert_eq!(unmangle_name("_ZN4testE"), "test");
    }

    #[test]
    fn unmangles_cpp_symbols() {
        assert_eq!(
            unmangle_name("_ZN5space3fooEibc"),
            "space::foo(int, bool, char)"
        );
    }

    #[test]
    fn preserves_unmangled_symbols() {
        assert_eq!(
            unmangle_name("not_a_mangled_symbol"),
            "not_a_mangled_symbol"
        );
    }

    #[test]
    fn preserves_symbols_that_fail_cpp_parsing() {
        assert_eq!(unmangle_name("_Z"), "_Z");
    }

    #[test]
    fn resets_weight_baseline_between_profiles() {
        let _test = PROFILE_TEST_LOCK.lock().unwrap();
        let engine = Engine::default();
        let mut store = Store::new(&engine, ());

        ProfilerBuilder::new(&mut store).profile(|_| {
            *LAST_WEIGHT.lock().unwrap() = 100;
        });
        ProfilerBuilder::new(&mut store).profile(|_| {
            let last_weight = *LAST_WEIGHT.lock().unwrap();
            assert_eq!(last_weight, 0);
        });
    }

    #[test]
    fn profiles_when_error_backtraces_are_disabled() {
        let _test = PROFILE_TEST_LOCK.lock().unwrap();
        let mut config = Config::new();
        config.epoch_interruption(true).wasm_backtrace(false);
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(&engine, "(module (func $work (export \"work\")))").unwrap();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let work = instance
            .get_typed_func::<(), ()>(&mut store, "work")
            .unwrap();

        let (_, data) = ProfilerBuilder::new(&mut store).profile(|store| {
            // Trigger a sample at function entry without waiting for a tick.
            engine.increment_epoch();
            work.call(store, ()).unwrap();
        });

        assert!(data.into_collapsed_stacks().to_string().contains("work"));
    }

    #[test]
    fn rejects_concurrent_profile_calls() {
        let _test = PROFILE_TEST_LOCK.lock().unwrap();
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).unwrap();
        let mut first_store = Store::new(&engine, ());
        let mut second_store = Store::new(&engine, ());

        thread::scope(|scope| {
            let first_profile = scope.spawn(|| {
                ProfilerBuilder::new(&mut first_store).profile(|_| {
                    thread::sleep(Duration::from_millis(10));
                });
            });

            thread::sleep(Duration::from_millis(1));

            let second_profile = scope.spawn(|| {
                ProfilerBuilder::new(&mut second_store).profile(|_| {});
            });

            first_profile.join().unwrap();
            assert!(second_profile.join().is_err());
        });
    }
}
