use std::io::Write;

use wasmprof::wasmprof;
use wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};

fn main() {
    let args = std::env::args();
    if args.len() != 2 {
        panic!("USAGE: wasmprof file.wasm")
    }

    let filename = args.last().unwrap();
    let mut config = Config::default();
    config.epoch_interruption(true);
    let engine = Engine::new(&config).unwrap();

    let module = Module::from_file(&engine, filename).unwrap();
    let mut store = Store::new(&engine, ());

    let instance = Instance::new(store.as_context_mut(), &module, &[]).unwrap();

    let func = instance
        .get_typed_func::<i64, i64>(store.as_context_mut(), "fib")
        .unwrap();
    let (res, _) = wasmprof(
        100,
        &mut store,
        wasmprof::WeightUnit::Nanoseconds,
        |mut store| {
            for _ in 0..10 {
                func.call(&mut store, 40).unwrap();
            }
        },
    );

    let mut result_file = std::fs::File::create("wasmprof.data").unwrap();
    write!(&mut result_file, "{}", res.into_collapsed_stacks()).unwrap();
}
