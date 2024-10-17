use std::fs::File;
use std::io::Write;
use wasmprof::{ProfilerBuilder, WeightUnit};
use wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::default();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
        (module
            (export "fib" (func $fib))
            (func $fib (param $n i32) (result i32)
             (if (result i32)
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
    )?;

    let mut store = Store::new(&engine, ());

    let (result, profile_data) = ProfilerBuilder::new(&mut store)
        .frequency(1000)
        .weight_unit(WeightUnit::Nanoseconds)
        .profile(|store| {
            let instance = Instance::new(store.as_context_mut(), &module, &[])?;
            let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")?;
            func.call(store.as_context_mut(), 40)
        });

    println!("Fibonacci(40) = {}", result?);

    // Convert profile data to speedscope format
    let speedscope_file = profile_data.to_speedscope(Some("Fibonacci Profile".to_string()));
    let json_output = speedscope_file.to_json()?;

    // Write the JSON to a file
    let mut file = File::create("fibonacci_profile.speedscope.json")?;
    file.write_all(json_output.as_bytes())?;

    println!("Speedscope profile saved to fibonacci_profile.speedscope.json");

    Ok(())
}
