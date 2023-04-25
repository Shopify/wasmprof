# wasmprof

A library that allows to profile code running inside of wasmtime

## How to use it

```rust
// First you want to create a `ProfilerBuilder` like so:
// Here we are assuming that you have a `Wasmtime::Store` to pass to the builder.
let builder = ProfilerBuilder::new(&mut store);

// Then you can set the frequency at which it's going to sample and the kind of weight to use:
// Here we are setting the frequency to 1000 (sampling 1000 in a second) and we chose `Fuel` as the weight
let builder = builder
    .frequency(1000)
    .weight_unit(wasmprof::WeightUnit::Fuel);

// finally we can start profiling
builder.profile(|store| {
    // here you would invoke some wasm function though wasmtime
    // like:
    let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")
                .unwrap();
    func.call(store.as_context_mut(), 40).unwrap()
})
```

A complete example can be found in the examples folder.

