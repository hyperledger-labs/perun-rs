#![no_main]
#![no_std]
#![feature(default_alloc_error_handler)]

mod channel;

use core::cell::RefCell;

use cortex_m::{interrupt::Mutex, peripheral::SYST};
use cortex_m_rt::{entry, exception};
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
        rng::Rng,
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
const SERVER_PORT: u16 = 1337;
const CIDR_PREFIX_LEN: u8 = 24;
const MAC_ADDRESS: EthernetAddress = EthernetAddress([0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF]);

static TIME: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));

type LedOutputPin<const N: u8> = gpio::Pin<'B', N, gpio::Output<gpio::PushPull>>;

fn get_ethemeral_port(rng: &mut Rng) -> u16 {
    const MIN: u16 = 49152;
    const MAX: u16 = 65535;
    // Note: This is not evenly distributed but sufficient for what we need.
    MIN + (rng.next_u32() as u16) % (MAX - MIN)
}

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

    // Connect to the server IP. Does not wait for the handshake to finish.
    let (socket, cx) = iface.get_socket_and_context::<TcpSocket>(handle);
    socket
        .connect(
            cx,
            (IpAddress::from(SERVER_IP_ADDRESS), SERVER_PORT),
            (IpAddress::Unspecified, get_ethemeral_port(hw_rng)),
        )
        .unwrap();

    // main application loop
    let mut last_toggle_time = 0;
    let mut greeted = false;
    loop {
        // Get the current time
        let time: u64 = cortex_m::interrupt::free(|cs| *TIME.borrow(cs).borrow());

        // Poll on the network stack
        match iface.poll(Instant::from_millis(time as i64)) {
            Ok(_) => {}
            Err(_) => red_led.set_high(),
        }

        let socket = iface.get_socket::<TcpSocket>(handle);

        // echo service to test sending and receiving of data. This echo service
        // will break if the other side does not read from the socket in time.
        // Since this is only intended for testing it should be fine. If it
        // would be a problem we could query the amount of available rx and tx
        // buffer space and only read then write that amount to not panic at one
        // of the unwraps below.
        if socket.can_send() && !greeted {
            socket
                .send_slice("Write anything and I'll reply\n".as_bytes())
                .unwrap();
            greeted = true;
        }
        if socket.can_recv() {
            let mut buf = [0u8; 128];
            socket.recv_slice(&mut buf).unwrap();
            socket.send_slice("Reply: ".as_bytes()).unwrap();
            socket.send_slice(&buf).unwrap();
            green_led.toggle();
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
