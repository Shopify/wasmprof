use wasmprof::ProfilerBuilder;
use wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};

fn main() {
    let mut config = Config::default();
    config.epoch_interruption(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"
        (module
            (export "fib" (func $fib))
            (func $fib (param $n i32) (result i32)
             (if
              (result i32)
              (i32.lt_s
               (local.get $n)
               (i32.const 2)
              )
              (then
               (i32.const 1)
              )
              (else
               (i32.add
                (call $fib
                 (i32.sub
                  (local.get $n)
                  (i32.const 2)
                 )
                )
                (call $fib2
                 (i32.sub
                  (local.get $n)
                  (i32.const 1)
                 )
                )
               )
              )
             )
            )
            (func $fib2 (param $n i32) (result i32)
             (if
              (result i32)
              (i32.lt_s
               (local.get $n)
               (i32.const 2)
              )
              (then
               (i32.const 1)
              )
              (else
               (i32.add
                (call $fib2
                 (i32.sub
                  (local.get $n)
                  (i32.const 2)
                 )
                )
                (call $fib
                 (i32.sub
                  (local.get $n)
                  (i32.const 1)
                 )
                )
               )
              )
             )
            )
           )
        "#,
    )
    .unwrap();

    let mut store = Store::new(&engine, ());
    store.set_fuel(100000000000).unwrap();

    let (_, res) = ProfilerBuilder::new(&mut store)
        .frequency(100)
        .weight_unit(wasmprof::WeightUnit::Fuel)
        .profile(|store| {
            let instance = Instance::new(store.as_context_mut(), &module, &[]).unwrap();
            let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")
                .unwrap();
            func.call(store.as_context_mut(), 40).unwrap()
        });

    println!("{}", res.into_collapsed_stacks());
}
