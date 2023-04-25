use wasmprof::wasmprof;
use wasmtime::{Config, Engine, Instance, Module, Store, AsContextMut};

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
              (i32.lt_s
               (get_local $n)
               (i32.const 2)
              )
              (return
               (i32.const 1)
              )
             )
             (return
              (i32.add
               (call $fib
                (i32.sub
                 (get_local $n)
                 (i32.const 2)
                )
               )
               (call $fib2
                (i32.sub
                 (get_local $n)
                 (i32.const 1)
                )
               )
              )
             )
            )
            (func $fib2 (param $n i32) (result i32)
             (if
              (i32.lt_s
               (get_local $n)
               (i32.const 2)
              )
              (return
               (i32.const 1)
              )
             )
             (return
              (i32.add
               (call $fib2
                (i32.sub
                 (get_local $n)
                 (i32.const 2)
                )
               )
               (call $fib
                (i32.sub
                 (get_local $n)
                 (i32.const 1)
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
    store.add_fuel(100000000000).unwrap();

    let (_, res, _) = wasmprof(100, engine, &mut store, wasmprof::WeightUnit::Fuel, |store| {
        let instance = Instance::new(store.as_context_mut(), &module, &[]).unwrap();
        let func = instance
            .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")
            .unwrap();
        func.call(store.as_context_mut(), 40).unwrap();
    });

    println!("{}", res.into_collapsed_stacks());
}
