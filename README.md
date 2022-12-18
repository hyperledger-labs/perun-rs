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

# Run Example/Walkthrough (can be configured at the top with constants)
cargo run --example lowlevel_basic_channel -F secp256k1

# Compile without std (the example above requires std)
cargo build --target thumbv7em-none-eabi --no-default-features
```
