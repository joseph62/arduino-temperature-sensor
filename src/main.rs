#![no_std]
#![no_main]

use core::{fmt::Write, marker::PhantomData};

use arduino_hal::{
    Usart, delay_ms,
    hal::Atmega,
    port::{
        Pin,
        mode::{Floating, Input},
    },
    usart::UsartOps,
};

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

struct DHT11<INIT>(Pin<Input<Floating>>, PhantomData<INIT>);

struct Initialized;
struct Uninitialized;

impl DHT11<Uninitialized> {
    pub fn new(pin: Pin<Input<Floating>>) -> Self {
        Self(pin, PhantomData)
    }

    pub fn init(self) -> DHT11<Initialized> {
        delay_ms(1000); // Wait for 1s for the sensor to stabilize
        DHT11(self.0, PhantomData)
    }
}

#[derive(Debug)]
enum DHT11ReadingError {
    ParityFailure(DHT11Reading),
    SensorUnresponsive((Signal, Signal)),
    BadSignalInterpretation((Signal, Signal, Signal)),
}

#[derive(Debug)]
enum Signal {
    High,
    Low,
}

impl From<&Pin<Input<Floating>>> for Signal {
    fn from(value: &Pin<Input<Floating>>) -> Self {
        if value.is_high() {
            Self::High
        } else {
            Self::Low
        }
    }
}

impl DHT11<Initialized> {
    fn read_sensor_bit(&self) -> Result<u32, (Signal, Signal, Signal)> {
        // Gather
        let first = (&self.0).into();
        delay_ms(50);
        let second = (&self.0).into();
        delay_ms(28);
        let third = (&self.0).into();
        match (first, second, third) {
            (Signal::Low, Signal::High, Signal::Low) => Ok(0),
            (Signal::Low, Signal::High, Signal::High) => Ok(1),
            signals => Err(signals),
        }
    }
    pub fn read(mut self) -> (Self, Result<DHT11Reading, DHT11ReadingError>) {
        let mut reading = 0u32;
        let mut parity = 0u32;

        // Signal that we want to read
        let mut output = self.0.into_output();
        output.set_low();
        delay_ms(18);
        output.set_high();
        delay_ms(18);
        output.set_low();

        self.0 = output.into_floating_input();

        // read acknowledge
        delay_ms(80);
        let first = (&self.0).into();
        delay_ms(80);
        let second = (&self.0).into();

        match (first, second) {
            (Signal::Low, Signal::High) => {
                // acknowledged data pin activity
            }
            signals => return (self, Err(DHT11ReadingError::SensorUnresponsive(signals))),
        };

        delay_ms(5);
        // read temperature and humidity
        for bit_index in 0..32 {
            match self.read_sensor_bit() {
                Ok(bit) => {
                    reading = reading & (bit << bit_index);
                }
                Err(signals) => {
                    return (
                        self,
                        Err(DHT11ReadingError::BadSignalInterpretation(signals)),
                    );
                }
            }
        }

        // read parity
        for bit_index in 0..8 {
            match self.read_sensor_bit() {
                Ok(bit) => {
                    parity = parity & (bit << bit_index);
                }
                Err(signals) => {
                    return (
                        self,
                        Err(DHT11ReadingError::BadSignalInterpretation(signals)),
                    );
                }
            }
        }

        let temperature = (reading >> 16) << 16;
        let humidity = reading << 16;

        let reading = DHT11Reading {
            temperature: temperature as u16,
            humidity: humidity as u16,
        };

        if temperature + humidity != parity {
            return (self, Err(DHT11ReadingError::ParityFailure(reading)));
        }

        (self, Ok(reading))
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

    let mut sensor = DHT11::new(pins.d12.downgrade()).init();

    loop {
        arduino_hal::delay_ms(1000);
        let (ret_sensor, reading_result) = sensor.read();
        sensor = ret_sensor;
        let _result = write!(usart, "Sensor reading {:?}\n", reading_result);
    }
}
