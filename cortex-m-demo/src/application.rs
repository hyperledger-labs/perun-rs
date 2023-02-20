//! State machine for the application logic. We need to do networking during the
//! setup and application logic, so we need to do it while the main loop is
//! running. Additionally, some steps cannot finish immediately. Since we
//! (currently at least) don't have an async runtime in this demo the easiest
//! way to do this is to have a state machine for the setup and application
//! logic, too, which is contained in this module.

use rand_core::RngCore;
use smoltcp::{
    iface::{Interface, SocketHandle},
    phy::Device,
    socket::TcpSocket,
    wire::IpAddress,
};
use stm32_eth::hal::rng::Rng;

pub struct Config {
    pub server: (IpAddress, u16),
}

pub struct Application<'a> {
    state: ApplicationState,
    handle: SocketHandle,
    config: Config,
    rng: &'a mut Rng,
}

enum ApplicationState {
    InitialState,
    Connecting,
    Running,
}

pub enum Error {
    Network(smoltcp::Error),
}

impl From<smoltcp::Error> for Error {
    fn from(e: smoltcp::Error) -> Self {
        Self::Network(e)
    }
}

impl<'a> Application<'a> {
    pub fn new(handle: SocketHandle, rng: &'a mut Rng, config: Config) -> Self {
        Self {
            state: ApplicationState::InitialState,
            handle,
            config,
            rng,
        }
    }

    fn connect(&mut self, iface: &mut Interface<impl for<'d> Device<'d>>) -> Result<(), Error> {
        // Connect to the server IP. Does not wait for the handshake to finish.
        let (socket, cx) = iface.get_socket_and_context::<TcpSocket>(self.handle);
        socket.connect(
            cx,
            self.config.server,
            (IpAddress::Unspecified, get_ethemeral_port(&mut self.rng)),
        )?;

        self.state = ApplicationState::Connecting;
        Ok(())
    }

    fn wait_active_and_greet(
        &mut self,
        iface: &mut Interface<impl for<'d> Device<'d>>,
    ) -> Result<(), Error> {
        let socket = iface.get_socket::<TcpSocket>(self.handle);
        const GREETING: &str = "Write anything and I'll reply\n";
        if socket.is_active() && socket.can_send() && socket.send_capacity() >= GREETING.len() {
            socket.send_slice(GREETING.as_bytes())?;
            self.state = ApplicationState::Running
        }
        Ok(())
    }

    fn read_and_reply(
        &mut self,
        iface: &mut Interface<impl for<'d> Device<'d>>,
    ) -> Result<(), Error> {
        let socket = iface.get_socket::<TcpSocket>(self.handle);
        if socket.can_recv() && socket.can_send() && socket.send_capacity() >= 128 {
            let mut buf = [0u8; 128];
            socket.recv_slice(&mut buf)?;
            socket.send_slice("Reply: ".as_bytes())?;
            socket.send_slice(&buf)?;
        }
        Ok(())
    }

    // echo service to test sending and receiving of data. This echo service
    // will break if the other side does not read from the socket in time.
    // Since this is only intended for testing it should be fine. If it
    // would be a problem we could query the amount of available rx and tx
    // buffer space and only read then write that amount to not panic at one
    // of the unwraps below.
    pub fn poll(&mut self, iface: &mut Interface<impl for<'d> Device<'d>>) -> Result<(), Error> {
        match self.state {
            ApplicationState::InitialState => self.connect(iface),
            ApplicationState::Connecting => self.wait_active_and_greet(iface),
            ApplicationState::Running => self.read_and_reply(iface),
        }
    }
}

fn get_ethemeral_port(rng: &mut Rng) -> u16 {
    const MIN: u16 = 49152;
    const MAX: u16 = 65535;
    // Note: This is not evenly distributed but sufficient for what we need.
    MIN + (rng.next_u32() as u16) % (MAX - MIN)
}
