use wasmprof::ProfilerBuilder;
use wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};

fn main() {
    let mut config = Config::default();
    config.epoch_interruption(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module_path = env!("CARGO_MANIFEST_DIR").to_string()
        + "/target/wasm32-unknown-unknown/release/dummy-runtime.wasm";
    println!("{}", module_path);
    let module = Module::from_file(&engine, module_path).unwrap();
    let mut store = Store::new(&engine, ());
    store.add_fuel(100000000000).unwrap();
    let instance = Instance::new(store.as_context_mut(), &module, &[]).unwrap();

    let (_, res) = ProfilerBuilder::new(&mut store)
        .frequency(10000)
        .instance(instance.clone())
        .add_runtimes("dummy".to_string())
        .weight_unit(wasmprof::WeightUnit::Fuel)
        .profile(|store| {
            let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")
                .unwrap();
            func.call(store.as_context_mut(), 40).unwrap()
        });

    println!("{}", res.into_collapsed_stacks());
}
