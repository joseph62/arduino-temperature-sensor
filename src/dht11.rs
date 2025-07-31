use core::marker::PhantomData;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};

#[derive(Debug)]
pub struct DHT11Reading {
    pub temperature: u16,
    pub humidity: u16,
}

pub struct DHT11<INIT, I: InputPin, O: OutputPin, D: DelayNs>(I, O, D, PhantomData<INIT>);

pub struct Initialized;
pub struct Uninitialized;

#[derive(Debug)]
pub enum DHT11ReadingError {
    ParityFailure(DHT11Reading),
    SensorUnresponsive((Signal, Signal)),
    BadSignalInterpretation((Signal, Signal, Signal)),
    UnableToReadSignal,
    UnableToWriteSignal,
}

#[derive(Debug, Clone, Copy)]
pub enum Signal {
    High,
    Low,
}

impl<I: InputPin, O: OutputPin, D: DelayNs> DHT11<Uninitialized, I, O, D> {
    pub fn new(input: I, output: O, delay: D) -> Self {
        Self(input, output, delay, PhantomData)
    }

    pub fn init(mut self) -> DHT11<Initialized, I, O, D> {
        self.2.delay_ms(1000); // Wait for 1s for the sensor to stabilize
        DHT11(self.0, self.1, self.2, PhantomData)
    }
}

impl<I: InputPin, O: OutputPin, D: DelayNs> DHT11<Initialized, I, O, D> {
    fn read_signal(&mut self) -> Result<Signal, DHT11ReadingError> {
        match self.0.is_high() {
            Ok(true) => Ok(Signal::High),
            Ok(false) => Ok(Signal::Low),
            _ => Err(DHT11ReadingError::UnableToReadSignal),
        }
    }

    fn read_signal_after(&mut self, after_ms: u32) -> Result<Signal, DHT11ReadingError> {
        self.2.delay_ms(after_ms);
        Ok(self.read_signal()?)
    }

    fn read_sensor_bit(&mut self) -> Result<u32, DHT11ReadingError> {
        // A bit signal set always starts low.
        // The difference between 1 and 0 is how quickly the pin
        // goes from high to low. A 0 indicates the pin went from high
        // to low in 28ish ms. A 1 indicates the pin stayed high for 70 ms
        let first = self.read_signal()?;
        let second = self.read_signal_after(50)?;
        let third = self.read_signal_after(28)?;
        match (first, second, third) {
            (Signal::Low, Signal::High, Signal::Low) => Ok(0),
            (Signal::Low, Signal::High, Signal::High) => Ok(1),
            signals => Err(DHT11ReadingError::BadSignalInterpretation(signals)),
        }
    }

    fn read_sensor_bits(&mut self, number_of_bits: u32) -> Result<u32, DHT11ReadingError> {
        let mut result = 0;

        for bit_index in 0..number_of_bits.min(32) {
            result = result & (self.read_sensor_bit()? << bit_index);
        }

        Ok(result)
    }

    fn write_sensor_read_start(&mut self) -> Result<(), DHT11ReadingError> {
        if let Err(_) = self.1.set_low() {
            return Err(DHT11ReadingError::UnableToWriteSignal);
        }
        self.2.delay_ms(18);
        if let Err(_) = self.1.set_high() {
            return Err(DHT11ReadingError::UnableToWriteSignal);
        }

        Ok(())
    }

    pub fn read(&mut self) -> Result<DHT11Reading, DHT11ReadingError> {
        // Signal that we want to read by driving the data pin
        // low and the resetting high

        self.write_sensor_read_start()?;

        // read the acknowledge, echo response
        let first = self.read_signal_after(80)?;
        let second = self.read_signal_after(80)?;

        match (first, second) {
            (Signal::Low, Signal::High) => {
                // acknowledged data pin activity
            }
            signals => return Err(DHT11ReadingError::SensorUnresponsive(signals)),
        };

        // wait a bit to catch the signal timing right
        self.2.delay_ms(5);

        // read 16 signals as bits for temperature and then another 16 for humidity
        let temperature = self.read_sensor_bits(16)?;

        let humidity = self.read_sensor_bits(16)?;

        // read the next 8 signals as parity bits
        let parity = self.read_sensor_bits(8)?;

        let reading = DHT11Reading {
            temperature: temperature as u16,
            humidity: humidity as u16,
        };

        if temperature + humidity != parity {
            return Err(DHT11ReadingError::ParityFailure(reading));
        }

        Ok(reading)
    }
}
