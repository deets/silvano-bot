use core::sync::atomic::{AtomicI32, Ordering};

use embassy_sync::channel::Receiver;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{Duration, Instant, Ticker};
use esp_hal::{
    Async,
    i2c::master::{Error, I2c, Operation},
};
use esp_println::println;

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
    bus: I2c<'a, Async>,
    last_update: Instant,
    receiver: Receiver<'static, NoopRawMutex, Movement, 4>,
    state_sender: Sender<'static, NoopRawMutex, MD23State, 16>,
    last_control: (u8, u8),
}

const MD23_ADDRESS: u8 = 0x58;
const MD23_LEFT: u8 = 1;
const MD23_RIGHT: u8 = 0;
const MD23_ENC1: u8 = 2;
const MD23_ENC2: u8 = 6;
const MD23_MODE: u8 = 15;

static COUNT: AtomicI32 = AtomicI32::new(0);

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
    pub async fn new(
        bus: I2c<'a, Async>,
        receiver: Receiver<'static, NoopRawMutex, Movement, 4>,
        state_sender: Sender<'static, NoopRawMutex, MD23State, 16>,
    ) -> Self {
        let mut this = Self {
            bus,
            last_update: Instant::now(),
            receiver,
            state_sender,
            last_control: (0, 0),
        };
        println!("set_mode: {:?}", this.set_register_u8(MD23_MODE, 0).await);
        this
    }

    async fn drive(&mut self) {
        while !self.receiver.is_empty() {
            let movement = self.receiver.receive().await;
            self.last_update = Instant::now();
            self.set_motor(movement.left, movement.right).await.ok();
        }
        if self.last_update.elapsed() > Duration::from_millis(500) {
            self.set_motor(0.0, 0.0).await.ok();
        }
        let encoders = self.read_encoders().await.ok();
        let speeds = self.read_speeds().await.ok();
        if encoders.is_some() && speeds.is_some() {
            let state = MD23State {
                encoders: encoders.unwrap(),
                speeds: speeds.unwrap(),
                control: self.last_control,
            };
            self.state_sender.send(state).await;
        }
    }

    async fn set_motor(&mut self, left: f32, right: f32) -> Result<(), Error> {
        let mut count = COUNT.load(Ordering::Relaxed);
        count += 1;
        COUNT.store(count, Ordering::Relaxed);
        let left = (-left * 127.0 + 128.0) as u8;
        let right = (-right * 127.0 + 128.0) as u8;
        self.set_register_u8(MD23_LEFT, left).await?;
        self.set_register_u8(MD23_RIGHT, right).await
    }

    async fn set_register_u8(&mut self, reg: u8, value: u8) -> Result<(), Error> {
        self.bus
            .transaction_async(
                MD23_ADDRESS,
                &mut [Operation::Write(&[reg]), Operation::Write(&[value])],
            )
            .await
    }

    async fn read_encoders(&mut self) -> Result<(i32, i32), Error> {
        let mut encoder_values = [0; 4];
        let _ = self.bus.write_async(MD23_ADDRESS, &[MD23_ENC1]).await?;
        let _ = self
            .bus
            .read_async(MD23_ADDRESS, &mut encoder_values)
            .await?;
        let left = i32::from_be_bytes(encoder_values);
        let _ = self.bus.write_async(MD23_ADDRESS, &[MD23_ENC2]).await?;
        let _ = self
            .bus
            .read_async(MD23_ADDRESS, &mut encoder_values)
            .await?;
        let right = i32::from_be_bytes(encoder_values);
        Ok((left, right))
    }

    async fn read_speeds(&mut self) -> Result<(u8, u8), Error> {
        Ok((
            self.read_register_u8(MD23_LEFT).await?,
            self.read_register_u8(MD23_RIGHT).await?,
        ))
    }

    async fn read_register_u8(&mut self, reg: u8) -> Result<u8, Error> {
        let mut value = [0; 1];
        let _ = self.bus.write_async(MD23_ADDRESS, &[reg]).await;
        let _ = self.bus.read_async(MD23_ADDRESS, &mut value).await;
        Ok(value[0])
    }
}

#[embassy_executor::task]
pub async fn movement_task(mut controller: MovementController<'static>) -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        controller.drive().await;
        ticker.next().await;
    }
}
