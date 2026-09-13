use display::SilvanoBotDisplay;
use esp_idf_svc::hal::{
    delay::{FreeRtos, BLOCK},
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::*,
};

mod display;

const SSD1306_ADDRESS: u8 = 0x3c;

fn main() -> eyre::Result<()> {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rys/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().expect("Can't create Peripherals");
    let i2c = peripherals.i2c1;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    println!("Starting I2C SSD1306 test");

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let mut display = SilvanoBotDisplay::new(i2c);
    loop {
        // we are sleeping here to make sure the watchdog isn't triggered
        display.update();
        println!("update");
        FreeRtos::delay_ms(100);
    }
}
