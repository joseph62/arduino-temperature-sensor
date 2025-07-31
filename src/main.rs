#![no_std]
#![no_main]

mod dht11;
mod serial;

use core::fmt::Write;

use arduino_hal::delay_ms;

use panic_halt as _;

use crate::dht11::{DHT11, DHT11ReadingError};
use crate::serial::SerialWriter;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut usart = SerialWriter::new(arduino_hal::default_serial!(dp, pins, 57600));

    let mut sensor = DHT11::new(
        pins.d12.downgrade().into_pull_up_input(),
        pins.d11.downgrade().into_output(),
    )
    .init();

    loop {
        delay_ms(1000);
        let _result = match sensor.read() {
            Ok(reading) => write!(
                usart,
                "Sensor reading: Temperature {}° C, Humidity {}%\n",
                reading.temperature, reading.humidity
            ),
            Err(DHT11ReadingError::ParityFailure(reading)) => write!(
                usart,
                "Sensor reading: Temperature {}° C, Humidity {}%, parity check failure\n",
                reading.temperature, reading.humidity
            ),
            Err(DHT11ReadingError::SensorUnresponsive(ref readings)) => {
                write!(usart, "Sensor unresponsive: Readings {:?}\n", readings)
            }
            Err(DHT11ReadingError::BadSignalInterpretation(ref readings)) => {
                write!(
                    usart,
                    "Sensor response bit misinterpretation: Readings {:?}\n",
                    readings
                )
            }
        };
    }
}
