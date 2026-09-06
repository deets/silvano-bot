use core::sync::atomic::{AtomicI32, Ordering};

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Receiver;
use embassy_time::{Duration, Instant, Ticker};
use esp_hal::{Async, i2c::master::I2c};
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
}

// The MD23 is implemented/documented a bit weird.  Normally you have
// a 127 bit address that's on the wire is shifted one left (so no
// address can have the highest bit set!), and the least signifcant
// bit (LSB) is 0 for read and 1 for write. Then registers etc. are
// achieved by writing a byte for the address offset (or possibly
// several), and then in a second transaction write or read as many
// bytes as desired.  However the MD23 instead takes it's base address
// already in shifted form and calls this a register, setting the LSB
// to 1 for writing. Thus the payload/value is always just the next
// byte directly, for the cost of 16 addresses consumed of 127
// avaialble.
const MD23_ADDRESS: u8 = 0xB0 >> 1;

static COUNT: AtomicI32 = AtomicI32::new(0);

impl<'a> MovementController<'a> {
    pub fn new(
        bus: I2c<'a, Async>,
        receiver: Receiver<'static, NoopRawMutex, Movement, 4>,
    ) -> Self {
        Self {
            bus,
            last_update: Instant::now(),
            receiver,
        }
    }

    async fn drive(&mut self) {
        while !self.receiver.is_empty() {
            let movement = self.receiver.receive().await;
            println!("drive: {:?}", movement);
            self.last_update = Instant::now();
            self.set_motor(movement.left, movement.right).await;
        }
        if self.last_update.elapsed() > Duration::from_millis(500) {
            self.set_motor(0.0, 0.0).await;
        }
    }

    async fn set_motor(&mut self, left: f32, right: f32) {
        let mut count = COUNT.load(Ordering::Relaxed);
        count += 1;
        COUNT.store(count, Ordering::Relaxed);
        let left = (left * 127.0) as u8;
        let right = (right * 127.0) as u8;
        println!("set_motor: {}, {} {}", count, left, right);
        if let Err(e) = self.bus.write_async(MD23_ADDRESS, &[left]).await {
            println!("set_motor:left:error{:?}", e);
        }
        if let Err(e) = self.bus.write_async(MD23_ADDRESS + 1, &[right]).await {
            println!("set_motor:right:error{:?}", e);
        }
    }
}

#[embassy_executor::task]
pub async fn movement_task(mut controller: MovementController<'static>) -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    loop {
        controller.drive().await;
        ticker.next().await;
    }
}
