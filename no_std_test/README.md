# no_std_test

A quick crate which is no_std, and uses the `runtime_call_macro` proc macro crate.

In this folder, run:

```rust
rustup +nightly target add thumbv6m-none-eabi
cargo +nightly build --target thumbv6m-none-eabi
```

To ensure no_std works.

Run `cargo expand` to check the macro output.