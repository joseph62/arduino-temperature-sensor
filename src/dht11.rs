use core::marker::PhantomData;

use arduino_hal::{
    delay_ms,
    port::{
        Pin,
        mode::{Floating, Input},
    },
};

#[derive(Debug)]
pub struct DHT11Reading {
    pub temperature: u16,
    pub humidity: u16,
}

pub struct DHT11<INIT>(Pin<Input<Floating>>, PhantomData<INIT>);

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
    pub fn new(pin: Pin<Input<Floating>>) -> Self {
        Self(pin, PhantomData)
    }

    pub fn init(self) -> DHT11<Initialized> {
        delay_ms(1000); // Wait for 1s for the sensor to stabilize
        DHT11(self.0, PhantomData)
    }
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
    fn read_signal(&self) -> Signal {
        (&self.0).into()
    }

    fn read_signal_after(&self, after_ms: u32) -> Signal {
        delay_ms(after_ms);
        self.read_signal()
    }

    fn read_sensor_bit(&self) -> Result<u32, (Signal, Signal, Signal)> {
        // Gather
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

    pub fn read(mut self) -> (Self, Result<DHT11Reading, DHT11ReadingError>) {
        // Signal that we want to read by driving the data pin
        // low and the resetting high
        let mut output = self.0.into_output();
        output.set_low();
        delay_ms(18);
        output.set_high();

        self.0 = output.into_floating_input();

        // read the acknowledge, echo response
        let first = self.read_signal_after(80);
        let second = self.read_signal_after(80);

        match (first, second) {
            (Signal::Low, Signal::High) => {
                // acknowledged data pin activity
            }
            signals => return (self, Err(DHT11ReadingError::SensorUnresponsive(signals))),
        };

        // wait a bit to catch the signalling timing right
        delay_ms(5);

        // read 16 signals as bits for temperature and then another 16 for humidity
        let temperature = match self.read_sensor_bits(16) {
            Ok(bits) => bits,
            Err(err) => return (self, Err(err)),
        };

        let humidity = match self.read_sensor_bits(16) {
            Ok(bits) => bits,
            Err(err) => return (self, Err(err)),
        };

        // read the next 8 signals as parity bits
        let parity = match self.read_sensor_bits(8) {
            Ok(bits) => bits,
            Err(err) => return (self, Err(err)),
        };

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
