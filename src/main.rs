#![no_std]
#![no_main]

use core::fmt::Write;

use arduino_hal::{
    Usart,
    hal::Atmega,
    usart::{UsartOps, UsartWriter},
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

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut usart = SerialWriter::new(arduino_hal::default_serial!(dp, pins, 57600));

    let mut led = pins.d13.into_output();

    loop {
        led.toggle();
        arduino_hal::delay_ms(1000);
        write!(usart, "Hello, World!\n");
    }
}
