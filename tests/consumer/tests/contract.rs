// This is free and unencumbered software released into the public domain.

// Compile the same public API contracts with downstream dependency features.
#[path = "../../bindings.rs"]
mod bindings;
#[path = "../../capabilities.rs"]
mod capabilities;
#[path = "../../diagnostics.rs"]
mod diagnostics;
#[path = "../../filter.rs"]
mod filter;
#[path = "../../inputs.rs"]
mod inputs;
#[path = "../../numbers.rs"]
mod numbers;
#[path = "../../visit.rs"]
mod visit;
