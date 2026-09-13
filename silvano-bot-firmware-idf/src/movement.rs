use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use esp_idf_svc::hal::i2c::{I2cDriver, Operation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Movement {
    pub left: f32,
    pub right: f32,
}

pub fn parse_query_string_for_motor_movement(path: Option<&'_ str>) -> Option<Movement> {
    let path = path?;
    let query = path.split_once('?').map_or(path, |(_, q)| q);
    let query = query.split_once('#').map_or(query, |(q, _)| q);

    let mut left = None;
    let mut right = None;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        if let Some((key, val)) = pair.split_once('=') {
            match key.trim() {
                "left" => {
                    let parsed = decode_float(val)?;
                    left = Some(parsed);
                }
                "right" => {
                    let parsed = decode_float(val)?;
                    right = Some(parsed);
                }
                _ => {}
            }
        }
    }

    match (left, right) {
        (Some(left), Some(right)) => Some(Movement { left, right }),
        _ => None,
    }
}

fn decode_float(s: &str) -> Option<f32> {
    s.parse::<f32>().ok()
}

pub struct MovementController<'a> {
    bus: I2cDriver<'a>,
    last_update: Instant,
    command: Arc<Mutex<Option<Movement>>>,
}

const MD23_ADDRESS: u8 = 0x58;
const MD23_LEFT: u8 = 1;
const MD23_RIGHT: u8 = 0;
const MD23_ENC1: u8 = 2;
const MD23_ENC2: u8 = 6;
const MD23_MODE: u8 = 15;

pub struct MD23State {
    /// The values we've written the last time we
    /// actively controlled the motors
    pub control: (u8, u8),
    /// The read back actual speed values
    pub speeds: (u8, u8),
    /// Encoders read the last time
    pub encoders: (i32, i32),
}

impl<'a> MovementController<'a> {
    pub fn new(bus: I2cDriver<'a>, command: Arc<Mutex<Option<Movement>>>) -> Self {
        Self {
            bus,
            last_update: Instant::now(),
            command,
        }
    }

    pub fn drive(&mut self) -> eyre::Result<()> {
        let movement = self
            .command
            .lock()
            .map_err(|_| eyre::eyre!("Can't lock"))?
            .take();
        if let Some(movement) = movement {
            self.last_update = Instant::now();
            self.set_motor(movement.left, movement.right)?;
        }
        if self.last_update.elapsed() > Duration::from_millis(500) {
            self.set_motor(0.0, 0.0)?;
        }
        Ok(())
    }

    fn set_motor(&mut self, left: f32, right: f32) -> eyre::Result<()> {
        let left = (-left * 127.0 + 128.0) as u8;
        let right = (-right * 127.0 + 128.0) as u8;
        self.set_register_u8(MD23_LEFT, left)?;
        self.set_register_u8(MD23_RIGHT, right)
    }

    fn set_register_u8(&mut self, reg: u8, value: u8) -> eyre::Result<()> {
        self.bus
            .transaction(
                MD23_ADDRESS,
                &mut [Operation::Write(&[reg]), Operation::Write(&[value])],
                100000,
            )
            .map_err(|_| eyre::eyre!("I2C movement error"))
    }

    // fn read_encoders(&mut self) -> Result<(i32, i32), Error> {
    //     let mut encoder_values = [0; 4];
    //     let _ = self.bus.write_async(MD23_ADDRESS, &[MD23_ENC1])?;
    //     let _ = self.bus.read_async(MD23_ADDRESS, &mut encoder_values)?;
    //     let left = i32::from_be_bytes(encoder_values);
    //     let _ = self.bus.write_async(MD23_ADDRESS, &[MD23_ENC2])?;
    //     let _ = self.bus.read_async(MD23_ADDRESS, &mut encoder_values)?;
    //     let right = i32::from_be_bytes(encoder_values);
    //     Ok((left, right))
    // }

    // fn read_speeds(&mut self) -> Result<(u8, u8), Error> {
    //     Ok((
    //         self.read_register_u8(MD23_LEFT)?,
    //         self.read_register_u8(MD23_RIGHT)?,
    //     ))
    // }

    // fn read_register_u8(&mut self, reg: u8) -> Result<u8, Error> {
    //     let mut value = [0; 1];
    //     let _ = self.bus.write_async(MD23_ADDRESS, &[reg]);
    //     let _ = self.bus.read_async(MD23_ADDRESS, &mut value);
    //     Ok(value[0])
    // }
}
