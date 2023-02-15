#![no_main]
#![no_std]

use cortex_m_rt::entry;

// Panic handler
use panic_halt as _;
use stm32_eth::{
    dma::{RxRingEntry, TxRingEntry},
    hal::{
        gpio::{self, PinState},
        hal::digital::v2::IoPin,
        prelude::*,
    },
    stm32::Peripherals,
    EthPins,
};

#[cfg(not(feature = "std"))]
#[entry]
fn entry() -> ! {
    main();
    loop {}
}

type LedOutputPin<const N: u8> = gpio::Pin<'B', N, gpio::Output<gpio::PushPull>>;

fn main() {
    let peripherals = Peripherals::take().unwrap();

    // Configure clock rate for Ethernet
    let rcc = peripherals.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(32.MHz()).hclk(32.MHz()).freeze();

    // Split up GPIO pins (PB is used for both LEDs and Ethernet)
    let gpioa = peripherals.GPIOA.split();
    let gpiob = peripherals.GPIOB.split();
    let gpioc = peripherals.GPIOC.split();
    let gpiog = peripherals.GPIOG.split();

    // LEDs
    let mut green_led: LedOutputPin<0> = gpiob.pb0.into_output_pin(PinState::Low).unwrap();
    let mut blue_led: LedOutputPin<7> = gpiob.pb7.into_output_pin(PinState::Low).unwrap();
    let mut red_led: LedOutputPin<14> = gpiob.pb14.into_output_pin(PinState::Low).unwrap();

    blue_led.set_high();

    // Ethernet
    let eth_pins = EthPins {
        ref_clk: gpioa.pa1,
        crs: gpioa.pa7,
        tx_en: gpiog.pg11,
        tx_d0: gpiog.pg13,
        tx_d1: gpiob.pb13,
        rx_d0: gpioc.pc4,
        rx_d1: gpioc.pc5,
    };
    let mut rx_ring: [RxRingEntry; 16] = Default::default();
    let mut tx_ring: [TxRingEntry; 8] = Default::default();
    let parts = stm32_eth::PartsIn {
        mac: peripherals.ETHERNET_MAC,
        mmc: peripherals.ETHERNET_MMC,
        dma: peripherals.ETHERNET_DMA,
        ptp: peripherals.ETHERNET_PTP,
    };
    let ethernet = stm32_eth::new(parts, &mut rx_ring, &mut tx_ring, clocks, eth_pins).unwrap();
    ethernet.dma.enable_interrupt();

    green_led.set_high();
}
