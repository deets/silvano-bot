use std::sync::{Arc, Mutex};

use display::SilvanoBotDisplay;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay::FreeRtos,
        i2c::{I2cConfig, I2cDriver},
        peripherals::Peripherals,
        units::*,
    },
    http::{Method, server::EspHttpServer},
    io::Write,
    mdns::EspMdns,
    nvs::EspDefaultNvsPartition,
    wifi::{self, AccessPointConfiguration, AuthMethod, BlockingWifi, EspWifi},
};

use log::info;
use movement::{MovementController, parse_query_string_for_motor_movement};

mod display;
mod movement;

const SSID: &str = "silvano-bot";
const CHANNEL: u8 = 11;
const INDEX_HTML: &[u8] = include_bytes!("../../assets/index.html");

fn main() -> eyre::Result<()> {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rys/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take().expect("Can't create Peripherals");

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let movement_command = Arc::new(Mutex::new(None));
    let server_movement_command = movement_command.clone();
    let mut server = create_server()?;

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?.write_all(INDEX_HTML).map(|_| ())
    })?;
    server.fn_handler("/move", Method::Get, move |req| {
        let mut movement_guard = server_movement_command
            .lock()
            .expect("Can't lock movement_command");
        *movement_guard = parse_query_string_for_motor_movement(Some(req.uri()));
        req.into_ok_response().map(|_| ())
    })?;

    // Setup mDNS
    let mut mdns = EspMdns::take()?;
    mdns.set_hostname("silvano-bot")?;
    // Advertise the HTTP server
    mdns.add_service(Some("Silvano Bot HTTP Server"), "_http", "_tcp", 80, &[])?;

    let i2c = peripherals.i2c1;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let display_i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let motor_i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio13,
        peripherals.pins.gpio14,
        &config,
    )?;

    let mut display = SilvanoBotDisplay::new(display_i2c);
    let mut controller = MovementController::new(motor_i2c, movement_command.clone());
    loop {
        // we are sleeping here to make sure the watchdog isn't triggered
        display.update();
        controller.drive()?;
        FreeRtos::delay_ms(16);
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> eyre::Result<()> {
    // If instead of creating a new network you want to serve the page
    // on your local network, you can replace this configuration with
    // the client configuration from the http_client example.
    let wifi_configuration = wifi::Configuration::AccessPoint(AccessPointConfiguration {
        ssid: SSID.try_into().unwrap(),
        ssid_hidden: false,
        auth_method: AuthMethod::None,
        password: "".try_into().unwrap(),
        channel: CHANNEL,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    info!("Created Wi-Fi with WIFI_SSID `{SSID}`");

    Ok(())
}

const STACK_SIZE: usize = 10240;
fn create_server() -> eyre::Result<EspHttpServer<'static>> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    Ok(EspHttpServer::new(&server_configuration)?)
}
