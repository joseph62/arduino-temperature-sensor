use core::marker::PhantomData;

use arduino_hal::{
    delay_ms,
    port::{
        Pin,
        mode::{Input, Output, PullUp},
    },
};

#[derive(Debug)]
pub struct DHT11Reading {
    pub temperature: u16,
    pub humidity: u16,
}

pub struct DHT11<INIT>(Pin<Input<PullUp>>, Pin<Output>, PhantomData<INIT>);

pub struct Initialized;
pub struct Uninitialized;

#[derive(Debug)]
pub enum DHT11ReadingError {
    ParityFailure(DHT11Reading),
    SensorUnresponsive((Signal, Signal)),
    BadSignalInterpretation((Signal, Signal, Signal)),
}

#[derive(Debug, Clone, Copy)]
pub enum Signal {
    High,
    Low,
}

impl DHT11<Uninitialized> {
    pub fn new(input: Pin<Input<PullUp>>, output: Pin<Output>) -> Self {
        Self(input, output, PhantomData)
    }

    pub fn init(self) -> DHT11<Initialized> {
        delay_ms(1000); // Wait for 1s for the sensor to stabilize
        DHT11(self.0, self.1, PhantomData)
    }
}

impl DHT11<Initialized> {
    fn read_signal(&self) -> Signal {
        if self.0.is_high() {
            Signal::High
        } else {
            Signal::Low
        }
    }

    fn read_signal_after(&self, after_ms: u32) -> Signal {
        delay_ms(after_ms);
        self.read_signal()
    }

    fn read_sensor_bit(&self) -> Result<u32, (Signal, Signal, Signal)> {
        // A bit signal set always starts low.
        // The difference between 1 and 0 is how quickly the pin
        // goes from high to low. A 0 indicates the pin went from high
        // to low in 28ish ms. A 1 indicates the pin stayed high for 70 ms
        let first = self.read_signal();
        let second = self.read_signal_after(50);
        let third = self.read_signal_after(28);
        match (first, second, third) {
            (Signal::Low, Signal::High, Signal::Low) => Ok(0),
            (Signal::Low, Signal::High, Signal::High) => Ok(1),
            signals => Err(signals),
        }
    }

    fn read_sensor_bits(&self, number_of_bits: u32) -> Result<u32, DHT11ReadingError> {
        let mut result = 0;

        for bit_index in 0..number_of_bits.min(32) {
            match self.read_sensor_bit() {
                Ok(bit) => {
                    result = result & (bit << bit_index);
                }
                Err(signals) => {
                    return Err(DHT11ReadingError::BadSignalInterpretation(signals));
                }
            }
        }

        Ok(result)
    }

    fn write_sensor_read_start(&mut self) {
        self.1.set_low();
        delay_ms(18);
        self.1.set_high();
    }

    pub fn read(&mut self) -> Result<DHT11Reading, DHT11ReadingError> {
        // Signal that we want to read by driving the data pin
        // low and the resetting high

        self.write_sensor_read_start();

        // read the acknowledge, echo response
        let first = self.read_signal_after(80);
        let second = self.read_signal_after(80);

        match (first, second) {
            (Signal::Low, Signal::High) => {
                // acknowledged data pin activity
            }
            signals => return Err(DHT11ReadingError::SensorUnresponsive(signals)),
        };

        // wait a bit to catch the signal timing right
        delay_ms(5);

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
