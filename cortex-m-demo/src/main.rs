#![no_main]
#![no_std]
#![feature(default_alloc_error_handler)]

mod application;
mod bus;
mod channel;

use core::{borrow::Borrow, cell::RefCell};

use application::{Application, Config};
use bus::Bus;
use cortex_m::{interrupt::Mutex, peripheral::SYST};
use cortex_m_rt::{entry, exception};
use perun::{sig::Signer, wire::ProtoBufEncodingLayer, PerunClient};
use rand::{rngs::StdRng, SeedableRng};
use rand_core::RngCore;
use smoltcp::{
    iface::{InterfaceBuilder, NeighborCache},
    socket::{TcpSocket, TcpSocketBuffer},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use stm32_eth::{
    dma::{RxRingEntry, TxRingEntry},
    hal::{
        gpio::{self, PinState},
        hal::digital::v2::IoPin,
        prelude::*,
    },
    stm32::{CorePeripherals, Peripherals},
    EthPins,
};

// Panic handler
use panic_halt as _;

extern crate alloc;

// Global allocator
use embedded_alloc::Heap;
#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn entry() -> ! {
    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 2048;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    main();
    loop {}
}

const IP_ADDRESS: Ipv4Address = Ipv4Address::new(10, 0, 0, 2);
const SERVER_IP_ADDRESS: Ipv4Address = Ipv4Address::new(10, 0, 0, 1);
const SERVER_CONFIG_PORT: u16 = 1339;
const SERVER_PARTICIPANT_PORT: u16 = 1337;
const CIDR_PREFIX_LEN: u8 = 24;
const MAC_ADDRESS: EthernetAddress = EthernetAddress([0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF]);

static TIME: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));

type LedOutputPin<const N: u8> = gpio::Pin<'B', N, gpio::Output<gpio::PushPull>>;

fn main() {
    let peripherals = Peripherals::take().unwrap();
    let mut core_peripherals = CorePeripherals::take().unwrap();

    // Configure clock rate for Ethernet
    let rcc = peripherals.RCC.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(32.MHz())
        .hclk(32.MHz())
        .require_pll48clk()
        .freeze();

    // System Timer
    setup_systick(&mut core_peripherals.SYST);

    // Split up GPIO pins (PB is used for both LEDs and Ethernet)
    let gpioa = peripherals.GPIOA.split();
    let gpiob = peripherals.GPIOB.split();
    let gpioc = peripherals.GPIOC.split();
    let gpiog = peripherals.GPIOG.split();

    // LEDs
    let mut green_led: LedOutputPin<0> = gpiob.pb0.into_output_pin(PinState::Low).unwrap();
    let mut blue_led: LedOutputPin<7> = gpiob.pb7.into_output_pin(PinState::Low).unwrap();
    let mut red_led: LedOutputPin<14> = gpiob.pb14.into_output_pin(PinState::Low).unwrap();

    // Ethernet (PHY)
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
    let mut ethernet = stm32_eth::new(parts, &mut rx_ring, &mut tx_ring, clocks, eth_pins).unwrap();
    // ethernet.dma.enable_interrupt();

    // Random Number generation
    let hw_rng = &mut peripherals.RNG.constrain(&clocks);
    let mut crypt_rng = StdRng::seed_from_u64(hw_rng.next_u64());

    // Configure IP Interface: smoltcp v0.8.2
    // At the moment we need to use the old version because stm32-eth hasn't
    // updated its smoltcp dependency in the latest release (only on master).
    // See https://github.com/stm32-rs/stm32-eth/blob/master/CHANGELOG.md
    // We could either use the unreleased master branch of stm32-eth or use the
    // older 0.8.2 version of smoltcp.
    let ip_addr = IpCidr::new(IP_ADDRESS.into(), CIDR_PREFIX_LEN);
    let mut ip_addrs = [ip_addr];
    let mut neighbor_storage = [None; 16];
    let neighbor_cache = NeighborCache::new(&mut neighbor_storage[..]);
    let mut sockets: [_; 1] = Default::default();
    let mut iface = InterfaceBuilder::new(&mut ethernet.dma, &mut sockets[..])
        .random_seed(hw_rng.next_u64())
        .hardware_addr(HardwareAddress::Ethernet(MAC_ADDRESS))
        .ip_addrs(&mut ip_addrs[..])
        .neighbor_cache(neighbor_cache)
        .finalize();

    // Configure IP Interface: smoltcp v0.9.1 (untested)
    /*
    let mut config = Config::new();
    config.random_seed = hw_rng.next_u64();
    config.hardware_addr = Some(HardwareAddress::Ethernet(MAC_ADDRESS));
    let mut iface = Interface::new(config, &mut &mut ethernet.dma);
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::Ipv4(Ipv4Cidr::new(IP_ADDRESS, CIDR_PREFIX_LEN)))
            .unwrap()
    });
    */

    // Configure TCP socket (and allocate buffers)
    let mut server_rx_buffer = [0; 512];
    let mut server_tx_buffer = [0; 512];
    let socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut server_rx_buffer[..]),
        TcpSocketBuffer::new(&mut server_tx_buffer[..]),
    );
    let handle = iface.add_socket(socket);

    blue_led.set_high(); // Setup finished

    let config = Config {
        config_server: (IpAddress::from(SERVER_IP_ADDRESS), SERVER_CONFIG_PORT),
        other_participant: (IpAddress::from(SERVER_IP_ADDRESS), SERVER_PARTICIPANT_PORT),
        participants: ["Alice", "Bob"],
    };

    // Move the interface into a RefCell because we need a mutable reference in
    // the main loop below, in app and in bus. (Having one in app could be
    // avoided by always passing in the interface, but that is effort and will
    // cause problems when calling methods on the channel, which call functions
    // on the bus and thus need to mutably borrow the interface, too).
    let iface = &RefCell::new(iface);

    let bus = Bus { iface, handle };
    // We need/want randomness for signing and for generating the ephemeral
    // port numbers. Creating a new RNG from the one we got is the easiest
    // way to do so, allowing both to have ownership of a RNG though it is
    // not the same. According to
    // https://rust-random.github.io/book/guide-rngs.html the cost of doing
    // this is small, as initialization is fast and the RNGs internal state
    // is 136 bytes (StdRng currently uses ChaCha12).
    let mut rng2 = StdRng::seed_from_u64(hw_rng.next_u64());
    let signer = Signer::new(&mut rng2);
    let addr = signer.address();
    let client = PerunClient::new(ProtoBufEncodingLayer { bus }, signer);
    let mut app = Application::new(handle, config, rng2, addr, &client, green_led, iface);

    // main application loop
    let mut last_toggle_time = 0;
    let mut stop = false;
    loop {
        // Get the current time
        let time: u64 = cortex_m::interrupt::free(|cs| *TIME.borrow(cs).borrow());

        // Poll on the network stack
        match iface.borrow_mut().poll(Instant::from_millis(time as i64)) {
            Ok(_) => {}
            Err(_) => {}
        }

        if !stop {
            match app.poll() {
                Ok(_) => {}
                Err(_) => {
                    red_led.set_high();
                    stop = true;
                }
            }
        }

        // Toggle the blue LED every second
        if time > last_toggle_time + 1000 {
            blue_led.toggle();
            last_toggle_time = time;
        }
    }
}

fn setup_systick(syst: &mut SYST) {
    syst.set_reload(SYST::get_ticks_per_10ms() / 10);
    syst.enable_counter();
    syst.enable_interrupt();
}

#[exception]
fn SysTick() {
    cortex_m::interrupt::free(|cs| {
        let mut time = TIME.borrow(cs).borrow_mut();
        *time += 1;
    })
}
