use std::fs::File;
use std::io::Write;
use wasmprof::{ProfilerBuilder, WeightUnit};
use wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};

const STARTING_FUEL: u64 = u64::MAX;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::default();
    config.epoch_interruption(true);
    config.consume_fuel(true);
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
    store.set_fuel(STARTING_FUEL)?;  // Set initial fuel

    let (result, profile_data) = ProfilerBuilder::new(&mut store)
        .frequency(500_000)
        .weight_unit(WeightUnit::Fuel)
        .profile(|store| {
            let instance = Instance::new(store.as_context_mut(), &module, &[])?;
            let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")?;
            func.call(store.as_context_mut(), 30)
        });

    println!("Fibonacci(30) = {}", result?);
    
    // Calculate fuel consumed
    let fuel_consumed = STARTING_FUEL.saturating_sub(store.get_fuel().unwrap_or_default());
    println!("Fuel consumed: {}", fuel_consumed);

    // Convert profile data to speedscope format
    let speedscope_file = profile_data.to_speedscope(Some("Fibonacci Fuel Profile".to_string()));
    let json_output = speedscope_file.to_json()?;

    // Write the JSON to a file
    let mut file = File::create("fibonacci_fuel_profile.speedscope.json")?;
    file.write_all(json_output.as_bytes())?;

    println!("Speedscope profile saved to fibonacci_fuel_profile.speedscope.json");

    Ok(())
}
