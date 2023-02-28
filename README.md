# Perun Blockchain-Agnostic State Channels in Rust
Rust-perun allows using Perun channels (currently only 2-party payment channels)
on embedded devices, which is difficult when using Go. Since embedded devices
usually don't have enough computing power to watch the Ethereum blockchain the
Rust-perun repo uses an external service (implemented using Go-perun) for
Watching the blockchain for disputes and for funding a new channel.

## Getting started
```bash
# Initialize submodules
git submodule init
git submodule update

# (Optional) Install tools (if not done already).
cargo install cargo-flash

# Execute all tests
cargo test --all-features

# Run go-integration example/walkthrough (run in separate terminals)
# Ganache is optional, if it isn't running we're using the SimulatedBackend.
ganache -e 100000000000000 -s 1024 -b 5
cd examples/go-integration; go run . ; cd -
cargo run --example go-integration

# Run old Example/Walkthrough (can be configured at the top with constants)
cargo run --example lowlevel_basic_channel

# Compile without std (the example above requires std)
cargo build --target thumbv7em-none-eabi --no-default-features -F k256

# Compile example without std and run in qemu
# - does not have communication with Go
# - currently requires the nightly compiler (due to the chosen allocator)
# - `--release` is needed to reduce the binary size so it fits into FLASH
cargo +nightly run --example go-integration --target thumbv7m-none-eabi --no-default-features -F nostd-example --release
```

### Run (new) demo on real hardware
Configure the Laptop/Compuers Ethernet adapter to the static IP 10.0.0.1/24,
which is the hard-coded IP expected by the device.

For the communication to work the Jumpers JP6 and JP7 must be set (connected),
along (most likely) some others. See the board documentation if the application
runs on the device but cannot communicate.

```bash
# Initialize submodules
git submodule init
git submodule update

# The demo requires nightly, so either set that as a default or use +nightly in each cargo command.
rustup override set nightly-x86_64-unknown-linux-gnu

# (Optional) Install tools (if not done already).
cargo install cargo-flash

# (Optional) Run Ganache in Terminal 1
ganache -e 100000000000000 -s 1024 -b 5

# Run Go-side in Terminal 2
cd examples/go-integration; go run . ; cd -

# Compile and Flash to connected device
cargo flash --chip STM32F439ZITx --target thumbv7em-none-eabihf -p cortex-m-demo --release
```

#### Debugging
In addition to ganache and the go-side:
```bash
# Compile with debug information (I couldn't get custom profiles to work with cargo-flash)
# Release optimizations are required to run (otherwise it needs too much memory)
# Using this profile additionally adds debug information, though it is not perfect.
cargo build --target thumbv7em-none-eabihf -p cortex-m-demo --profile=release-with-debug

# Start openocd in Terminal 3
cd cortex-m-demo; openocd ; cd -

# Start gdb in Terminal 4 (alternative binary name: gdb-multiarch)
arm-none-eabi-gdb -q target/thumbv7em-none-eabihf/release-with-debug/cortex-m-demo
```

In gdb enter the following:
```bash
# Connect to openocd
target extended-remote :3333

# Set breakpoints for panics
break DefaultHandler
break HardFault
break rust_begin_unwind

# (Optional) enable semihosting if needed (at the moment it isn't, but it can be useful when using panic_semihosting)
monitor arm semihosting enable

# Flash compiled binary to device (don't forget to recompile and load after making changes)
load

# Use the normal gdb commands to set breakpoints or run the application

# To restart the application without flashing:
run
```

## Feature Flags
- `std` (default)
- `k256` (default) Use [`k256`](https://crates.io/crates/k256) for signatures
- `secp256k1` Use [`secp256k1`](https://crates.io/crates/secp256k1) for signatures (implies `std`)
