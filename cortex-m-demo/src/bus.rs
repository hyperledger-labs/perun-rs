use core::cell::RefCell;

use perun::wire::BytesBus;
use smoltcp::{
    iface::{Interface, SocketHandle},
    phy::Device,
    socket::TcpSocket,
};

pub struct Bus<'iface, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    pub iface: &'iface RefCell<Interface<'iface, DeviceT>>,
    pub participant_handle: SocketHandle,
    pub service_handle: SocketHandle,
}

impl<'iface, DeviceT> Bus<'iface, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    fn send(&self, handle: SocketHandle, msg: &[u8]) {
        let mut iface = self.iface.borrow_mut();
        let socket = iface.get_socket::<TcpSocket>(handle);
        // Note: In this implementation the entire message has to fit into the
        // tx buffer. To loosen that requirement you'd need some way to queue
        // half the data and resume later, which is not easily doable without
        // async afaict.
        let count_written = socket.send_slice(msg).unwrap();
        if count_written != msg.len() {
            panic!(
                "Could not send message, wrote {count_written}/{} bytes",
                msg.len()
            );
        }
    }
}

impl<'iface, DeviceT> BytesBus for Bus<'iface, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    fn send_to_watcher(&self, msg: &[u8]) {
        self.send(self.service_handle, msg)
    }

    fn send_to_funder(&self, msg: &[u8]) {
        self.send(self.service_handle, msg)
    }

    fn send_to_participant(
        &self,
        _sender: &perun::wire::Identity,
        _recipient: &perun::wire::Identity,
        msg: &[u8],
    ) {
        self.send(self.participant_handle, msg)
    }
}
