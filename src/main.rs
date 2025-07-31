#![no_std]
#![no_main]

use core::{fmt::Write, marker::PhantomData};

use arduino_hal::{Usart, delay_ms, hal::Atmega, port::Pin, usart::UsartOps};

use panic_halt as _;

struct SerialWriter<USART: arduino_hal::usart::UsartOps<Atmega, RX, TX>, RX, TX>(
    Usart<USART, RX, TX>,
);

impl<USART: UsartOps<Atmega, RX, TX>, RX, TX> SerialWriter<USART, RX, TX> {
    pub fn new(usart: Usart<USART, RX, TX>) -> Self {
        Self(usart)
    }
}

impl<USART: UsartOps<Atmega, RX, TX>, RX, TX> Write for SerialWriter<USART, RX, TX> {
    fn write_str(&mut self, str: &str) -> core::fmt::Result {
        for b in str.as_bytes() {
            self.0.write_byte(*b);
        }
        Ok(())
    }
}

struct DHT11<M, INIT>(Pin<M>, PhantomData<INIT>);

struct Initialized;
struct Uninitialized;

impl<M> DHT11<M, Uninitialized> {
    pub fn new(pin: Pin<M>) -> Self {
        Self(pin, PhantomData)
    }

    pub fn init(self) -> DHT11<M, Initialized> {
        delay_ms(1000); // Wait for 1s for the sensor to stabilize
        DHT11(self.0, PhantomData)
    }
}

#[derive(Debug)]
enum DHT11ReadingErrors {
    ParityFailure,
}

impl<M> DHT11<M, Initialized> {
    pub fn read(&self) -> Result<DHT11Reading, DHT11ReadingErrors> {
        let mut reading = 0u32;
        let mut parity = 0u8;

        // Some awesome reading steps

        let temperature = (reading >> 16) << 16;
        let humidity = reading << 16;

        if temperature + humidity != parity as u32 {
            return Err(DHT11ReadingErrors::ParityFailure);
        }

        Ok(DHT11Reading {
            temperature: reading as u16,
            humidity: (reading << 16) as u16,
        })
    }
}

#[derive(Debug)]
struct DHT11Reading {
    temperature: u16,
    humidity: u16,
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut usart = SerialWriter::new(arduino_hal::default_serial!(dp, pins, 57600));

    let mut led = pins.d13.into_output();
    let sensor = DHT11::new(pins.d12.downgrade()).init();

    loop {
        led.toggle();
        arduino_hal::delay_ms(1000);
        let _result = write!(usart, "Sensor reading {:?}\n", sensor.read());
    }
}
