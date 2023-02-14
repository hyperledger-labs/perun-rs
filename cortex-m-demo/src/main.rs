#![no_main]
#![no_std]

use cortex_m_rt::entry;

// Panic handler
use panic_halt as _;

#[cfg(not(feature = "std"))]
#[entry]
fn entry() -> ! {
    main();
    loop {}
}

fn main() {}
