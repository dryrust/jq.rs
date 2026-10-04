# Filter benchmarks

Run `cargo bench --locked --bench filter` on an otherwise idle machine.
Set `JQ_BENCH_ITERS` to change the default 1,000 measured iterations per case.
Each case also performs ten warmup iterations. The stable-Rust harness prints
elapsed time per iteration; repeat runs and record the compiler, target, and
feature flags with results. These are exploratory measurements, not CI gates.

Compilation cases include standard-library setup. Separate definition-loading
and native-registration cases help distinguish that setup from compilation.
The reuse pair applies the same nested selection to the same input; both
include cloning the owned input. Result destruction is included in timing.
The benchmarks deliberately select jaq, including when `jq` is also enabled.
