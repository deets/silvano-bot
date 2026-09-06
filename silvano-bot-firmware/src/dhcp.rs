use core::net::SocketAddr;
use core::str::FromStr;
use edge_nal_060::{UdpReceive as UdpReceive060, UdpSend as UdpSend060};
use edge_nal_embassy::UdpError;
use embassy_net::Stack;
use embassy_time::{Duration, Timer};
use embedded_io::ErrorType;
use esp_println::println;

/// I couldn't solve the leaky dependency of edge_nal being
/// used in two versions, so this simple wrapper just relays
/// the calls.
struct UdpSocketWrapper<'a, S>
where
    S: UdpSend060 + UdpReceive060,
{
    socket: &'a mut S,
}

impl<S> edge_nal::UdpSend for UdpSocketWrapper<'_, S>
where
    S: UdpSend060 + UdpReceive060 + ErrorType<Error = UdpError>,
{
    async fn send(
        &mut self,
        remote: core::net::SocketAddr,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.socket.send(remote, data).await
    }
}

impl<S> edge_nal::UdpReceive for UdpSocketWrapper<'_, S>
where
    S: UdpReceive060 + UdpSend060 + ErrorType<Error = UdpError>,
{
    async fn receive(&mut self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Self::Error> {
        self.socket.receive(buffer).await
    }
}

impl<S> ErrorType for UdpSocketWrapper<'_, S>
where
    S: UdpReceive060 + UdpSend060,
{
    type Error = UdpError;
}

#[embassy_executor::task]
pub async fn run_dhcp(stack: Stack<'static>, gw_ip_addr: &'static str) {
    use core::net::{Ipv4Addr, SocketAddrV4};

    use edge_dhcp::{
        io::{self, DEFAULT_SERVER_PORT},
        server::{Server, ServerOptions},
    };
    use edge_nal_060::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};

    let ip = Ipv4Addr::from_str(gw_ip_addr).expect("dhcp task failed to parse gw ip");

    let mut buf = [0u8; 1500];

    let mut gw_buf = [Ipv4Addr::UNSPECIFIED];

    let buffers = UdpBuffers::<3, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    let mut wrapped_bound_socket = UdpSocketWrapper {
        socket: &mut bound_socket,
    };
    loop {
        _ = io::server::run(
            &mut Server::<_, 64>::new_with_et(ip),
            &ServerOptions::new(ip, Some(&mut gw_buf)),
            &mut wrapped_bound_socket,
            &mut buf,
        )
        .await
        .inspect_err(|e| println!("DHCP server error: {e:?}"));
        Timer::after(Duration::from_millis(500)).await;
    }
}
