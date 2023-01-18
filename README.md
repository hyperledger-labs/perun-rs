# Perun Blockchain-Agnostic State Channels in Rust
Rust-perun allows using Perun channels (currently only 2-party payment channels)
on embedded devices, which is difficult when using Go. Since embedded devices
usually don't have enough computing power to watch the Ethereum blockchain the
Rust-perun repo uses an external service (implemented using Go-perun) for
Watching the blockchain for disputes and for funding a new channel.

## Getting started
```bash
# Execute all tests
cargo test --all-features

# Run go-integration example/walkthrough (run in separate terminals)
cd examples/go-integration; go run . ; cd -
cargo run --example go-integration

# Run Example/Walkthrough (can be configured at the top with constants)
cargo run --example lowlevel_basic_channel

# Compile without std (the example above requires std)
cargo build --target thumbv7em-none-eabi --no-default-features -F k256

# Compile example without std and run in qemu
# - does not have communication with Go
# - currently requires the nightly compiler (due to the chosen allocator)
# - `--release` is needed to reduce the binary size so it fits into FLASH
cargo +nightly run --example go-integration --target thumbv7m-none-eabi --no-default-features -F nostd-example --release
```

## Feature Flags
- `std` (default)
- `k256` (default) Use [`k256`](https://crates.io/crates/k256) for signatures
- `secp256k1` Use [`secp256k1`](https://crates.io/crates/secp256k1) for signatures (implies `std`)
