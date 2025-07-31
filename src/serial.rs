use core::fmt::Write;

use arduino_hal::{Usart, hal::Atmega, usart::UsartOps};

pub struct SerialWriter<USART: arduino_hal::usart::UsartOps<Atmega, RX, TX>, RX, TX>(
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
