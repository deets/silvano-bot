//! Embassy access point
//!
//! - creates an open access-point with SSID `esp-radio`
//! - DHCP is enabled so there's no need to configure a static IP
//! - connect to the AP `esp-radio` and open http://1.1.1.1:8080/ in your browser
//!
//! On Android you might need to choose _Keep Accesspoint_ when it tells you the
//! WiFi has no internet connection, Chrome might not want to load the URL - you
//! can use a shell and try `curl` and `ping`

#![no_std]
#![no_main]
#![feature(int_format_into)]

use core::{fmt::NumBuffer, net::Ipv4Addr, str::FromStr};
use embassy_executor::Spawner;
use embassy_net::{
    IpListenEndpoint, Ipv4Cidr, Runner, StackResources, StaticConfigV4, tcp::TcpSocket,
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::{Sender as WatchSender, Watch};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::Write;
use esp_alloc as _;
use esp_backtrace as _;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{clock::CpuClock, ram, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_radio::{
    Controller,
    wifi::{AccessPointConfig, ModeConfig, WifiApState, WifiController, WifiDevice, WifiEvent},
};
use serde::Serialize;
use silvano_bot_firmware::display::SilvanoBotDisplay;
use silvano_bot_firmware::display::display_task;
use silvano_bot_firmware::movement::{
    MD23State, Movement, movement_task, parse_query_string_for_motor_movement,
};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const BLOCK_SIZE: usize = 1024;
const INDEX_HTML: &[u8] = include_bytes!("../../assets/index.html");
const GW_IP_ADDR_ENV: Option<&'static str> = option_env!("GATEWAY_IP");
static CONTROL_WATCH: StaticCell<Watch<NoopRawMutex, Movement, 4>> = StaticCell::new();
static STATE_CHANNEL: StaticCell<Channel<NoopRawMutex, MD23State, 16>> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let md23_i2c_bus = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(esp_hal::time::Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO14)
    .with_sda(peripherals.GPIO13)
    .into_async();
    let control_watch = Watch::new();
    let static_control_watch = CONTROL_WATCH.init(control_watch);

    let state_channel = Channel::new();
    let static_state_channel = STATE_CHANNEL.init(state_channel);

    let movement_controller = silvano_bot_firmware::movement::MovementController::new(
        md23_i2c_bus,
        static_control_watch
            .receiver()
            .expect("Can't get control receiver"),
        static_state_channel.sender(),
    )
    .await;

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    // Needs to be after starting RTOS!
    let display_i2c_bus = esp_hal::i2c::master::I2c::new(
        peripherals.I2C1,
        esp_hal::i2c::master::Config::default().with_frequency(esp_hal::time::Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO22)
    .with_sda(peripherals.GPIO21)
    .into_async();
    let display = SilvanoBotDisplay::new(display_i2c_bus, static_state_channel.receiver()).await;

    let esp_radio_ctrl = &*mk_static!(Controller<'static>, esp_radio::init().unwrap());

    let (controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_ctrl, peripherals.WIFI, Default::default()).unwrap();

    let device = interfaces.ap;

    let gw_ip_addr_str = GW_IP_ADDR_ENV.unwrap_or("192.168.2.1");
    let gw_ip_addr = Ipv4Addr::from_str(gw_ip_addr_str).expect("failed to parse gateway ip");

    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(gw_ip_addr, 24),
        gateway: Some(gw_ip_addr),
        dns_servers: Default::default(),
    });

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        device,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(movement_task(movement_controller)).ok();
    spawner.spawn(display_task(display)).ok();
    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    spawner
        .spawn(silvano_bot_firmware::dhcp::run_dhcp(stack, gw_ip_addr_str))
        .ok();

    let mut rx_buffer = [0; 1536];
    let mut tx_buffer = [0; 1536];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    println!(
        "Connect to the AP `esp-radio` and point your browser to http://{gw_ip_addr_str}:8080/"
    );
    println!("DHCP is enabled so there's no need to configure a static IP, just in case:");
    while !stack.is_config_up() {
        Timer::after(Duration::from_millis(100)).await
    }
    stack
        .config_v4()
        .inspect(|c| println!("ipv4 config: {c:?}"));

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
    loop {
        println!("Wait for connection...");
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 8080,
            })
            .await;
        println!("Connected...");

        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }
        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(len) => {
                    let to_print =
                        unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

                    if to_print.contains("\r\n\r\n") {
                        dispatch(to_print, &mut socket, static_control_watch.sender()).await;
                        break;
                    }

                    pos += len;
                }
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
        }
        socket.close();
        socket.abort();
    }
}

async fn dispatch(
    request: &str,
    socket: &mut TcpSocket<'_>,
    movement_sender: WatchSender<'static, NoopRawMutex, Movement, 4>,
) {
    if let Err(e) = {
        if request.starts_with("GET / ") {
            send_http_response(socket, Response::Buffer(INDEX_HTML), 200).await
        } else if request.starts_with("GET /move?") {
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            match req.parse(request.as_bytes()) {
                Ok(_) => {
                    if let Some(movement) = parse_query_string_for_motor_movement(req.path) {
                        movement_sender.send(movement);
                    }
                    send_http_response(socket, Response::Movement((0, 0)), 200).await
                }
                Err(_) => send_http_response(socket, Response::Error, 500).await,
            }
        } else {
            send_http_response(socket, Response::Unknown, 400).await
        }
    } {
        println!("write error: {:?}", e);
    }
}

#[derive(Serialize)]
enum Response<'a> {
    Unknown,
    Movement((i32, i32)),
    Error,
    Buffer(&'a [u8]),
}

async fn write_time_header(socket: &mut TcpSocket<'_>) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = NumBuffer::new();
    let timestamp = Instant::now().as_micros();
    socket.write_all(b"Timestamp: ").await?;
    let timestamp_buf = timestamp.format_into(&mut buf);
    socket.write_all(timestamp_buf.as_bytes()).await?;
    socket.write_all(b"\r\n").await
}

async fn send_http_response<'a>(
    socket: &mut TcpSocket<'_>,
    response: Response<'a>,
    status: usize,
) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = NumBuffer::new();
    socket.write_all(b"HTTP/1.0 ").await?;
    let status = status.format_into(&mut buf);
    socket.write_all(status.as_bytes()).await?;
    // This must be big enough for all but the index.html,
    // otherwise we crash.
    let mut payload = [0u8; 1024];

    let (buffer, len) = match response {
        Response::Buffer(buf) => {
            socket
                .write_all(b" OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: ")
                .await?;
            (buf, buf.len())
        }
        _ => {
            socket
                .write_all(b" OK\r\nContent-Type: application/json\r\nContent-Length: ")
                .await?;
            let len = serde_json_core::to_slice(&response, &mut payload[..]).unwrap();
            (payload.as_slice(), len)
        }
    };

    let content_length = len.format_into(&mut buf);
    socket.write_all(content_length.as_bytes()).await?;
    socket.write_all(b"\r\n").await?;

    write_time_header(socket).await?;

    // Extra CRLF to terminate headers
    socket.write_all(b"\r\n").await?;

    let mut written = 0;
    while written < len {
        let until = core::cmp::min(written + BLOCK_SIZE, len);
        socket.write_all(&buffer[written..until]).await?;
        written += 1024;
        socket.flush().await?;
    }
    Ok(())
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_radio::wifi::ap_state() {
            WifiApState::Started => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config =
                ModeConfig::AccessPoint(AccessPointConfig::default().with_ssid("esp-radio".into()));
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
