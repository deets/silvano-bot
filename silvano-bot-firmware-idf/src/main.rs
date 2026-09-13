use display::SilvanoBotDisplay;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay::{FreeRtos, BLOCK},
        i2c::{I2cConfig, I2cDriver},
        peripherals::Peripherals,
        units::*,
    },
    nvs::EspDefaultNvsPartition,
    wifi::{self, AccessPointConfiguration, AuthMethod, BlockingWifi, EspWifi},
};
use log::info;

mod display;

const SSID: &str = "silvano-bot";
const CHANNEL: u8 = 11;

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

    let i2c = peripherals.i2c1;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let mut display = SilvanoBotDisplay::new(i2c);
    loop {
        // we are sleeping here to make sure the watchdog isn't triggered
        display.update();
        FreeRtos::delay_ms(100);
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
