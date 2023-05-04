# wasmprof: a tool for profiling code running in wasmtime

## What is a profiler? like really?

A profiler is a tool that helps you understand how your program is running, where it's spending time and what it's doing.
There are multiple kind of profilers, each with their own tradeoffs. Some prefer to instrument the code, others prefer to sample the code.
In this blogpost, we'll focus on sampling profilers, as wasmprof is one of them.

Having said that, what are the basic actions that a sampling profiler does?

- Every so often it wakes up (in some way). How often it wakes up is usually configurable and it's called the sampling rate, or sampling frequency.
- When it wakes up it collects information about the current state of the program. This information is called a sample.
- It then stores the sample somewhere, usually in some kind of resizable buffer.

The samples are then used to generate a report, which is the output of the profiler. The report can be in different formats, but their content is usually the same: a list of functions and how much time was spent in each of them.

## How does wasmprof do it?

wasmprof is a sampling profiler that works on WebAssembly programs. It's written in Rust and it's built on top of [wasmtime](https://github.com/bytecodealliance/wasmtime). wasmtime has some really nice apis which can be used to implement a profiler. The first such feature is epoch-based
interruption. How does this work? Taken from the wasmtime docs:

```
There is a global “epoch”, which is a counter that divides time into arbitrary periods (or epochs). This counter lives on the Engine and can be incremented by calling Engine::increment_epoch. Epoch-based instrumentation works by setting a “deadline epoch”. The compiled code knows the deadline, and at certain points, checks the current epoch against that deadline. It will yield if the deadline has been reached.
```

What this means is that we can increase the epoch and the running program inside of wasmtime will yield back to the executor once the deadline is
reached. Furthermore, we can register a custom callback function that is called when the epoch deadline is reached! This is exactly what wasmprof does.
It registers a callback function that is called when the epoch deadline is reached. This callback function is responsible for collecting the sample and storing it in a `Vec`.

But wait, how do we increase the epoch? For that we rely on different strategies depending on the OS we are on. On unix-like platforms we use setitimer,
which is a system call that allows us to set a timer that will send a signal to the process when it expires. We then register a signal handler that will
increase the epoch (which is signal safe, thank you wasmtime!). On windows, we spawn a thread that will sleep for the sampling rate and then increase the epoch, we use a mix of actually sleeping and spinning to get a precise sampling rate (windows does not allow sleeps with a precision higher than 1ms).

## What is this sample you talk about?

Most profilers when sampling the program really just take a snapshop of the current stacktrace. Luckily for us, wasmtime has a really nice
api for this too. We can use `WasmBacktrace` from the executor to get the current stacktrace of the running program. We use this in wasmprof to get the current stacktrace and then we store it in some global `Vec` so that we can process it later to create a report.

## How do we get the report?

Currently wasmprof has its own data structures to keep track of the information that any report may need to be constructed. This is stored as:

```rust
pub struct ProfileData {
    frames: Vec<String>,
    samples: Vec<Vec<usize>>,
    weights: Option<Vec<u128>>,
}
```

The `frames` are pretty much all the functions that we have seen on any stacktrace we have collected (so that we do not store the same function names multiple times). The `samples` are the actuall stacktraces that we have collected, where the `usize` is just an index into `frames`. Each sample may also have a weight associated with it, which is the importance of that sample. This for example could be the time since the last sample was collected.

We use this data to produce file formats that other programs (like speedscope, pprof, etc) can understand. We are just at the beginning of this journey, so we only support Brendan Gregg's collapsed stack format, which is explained [here](https://github.com/jlfwong/speedscope/wiki/Importing-from-custom-sources#brendan-greggs-collapsed-stack-format). This can be fed to speedscope to get nice flamegraphs.

## The different kind of weights

We support 2 ways of weighing each sample:

- by time: this is the time since the last sample was collected. This is the default and it's what most profilers do
- by fuel: this is the amount of fuel that was consumed since the last sample was collected

Time is fairly straigthforward (if you just ignore all the bad things that can happen with time, but let's ignore those). Fuel is a concept specific
to wasmtime, which allows you to limit the amount of computation that a program can do. At Shopify, for example, we use fuel to limit
how much computations a Function can do.

But it's fairly hard to understand where you're program is spending its fuel. This is where wasmprof comes in. By using the fuel as a weight, we can
understand how fuel is used!

## Looking at the future

wasmprof already does a good job at being a profiler for programs running inside of wasmtime, but it's only particularly insightful when the program
runs some native code. Once you run JS, Ruby, etc, in wasm, you can't really see what's going on inside of those programs, as `WasmBacktrace` will only
show the internals of each language runtime. This is fairly common problem for native profilers, perf does not know anything about Ruby code for example.

Can we solve it? I think we can. There are few ideas that we are exploring, which are all based on the assumption that each runtime tracks its own stacktrace:

- allow each runtime to expose a function that returns its own stacktrace. We call this function from wasmprof and we store the stacktrace that we get back. This is fairly easy to do, but it requires each runtime to expose this function.
- if we can't make that work, we will allow each runtime to register a plugin in wasmprof that is able to get the memory of the runtime and extract the stacktrace from it. This is a bit more involved, but it's our backup plan.

The idea is that wasmprof will be an universal profiler for any kind of language that can run inside of wasmtime! Exciting times ahead!

## How do I use it?

The best way to understand how to use wasmprof is to look at the [examples](https://github.com/Shopify/wasmprof/tree/master/examples). But here's a quick rundown:

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
let report = builder.profile(|store| {
    // here you would invoke some wasm function though wasmtime, something like this:
    let func = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "fib")
                .unwrap();
    func.call(store.as_context_mut(), 40).unwrap()
})

println!("{}", report.into_collapsed_stacks());
```


