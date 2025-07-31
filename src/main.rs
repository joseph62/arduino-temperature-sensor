#![no_std]
#![no_main]

use arduino_hal::{hal::Atmega, usart::UsartOps, Usart};

use panic_halt as _;

fn write_usart<B: UsartOps<Atmega, P, T>, P, T>(usart: &mut Usart<B, P, T>, bytes: &[u8]) {
    for b in bytes {
        usart.write_byte(*b);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut usart =    arduino_hal::default_serial!(dp, pins, 57600);

    let mut led = pins.d13.into_output();

    loop {
        led.toggle();
        arduino_hal::delay_ms(1000);
        write_usart(&mut usart, b"Hello, World!\n" );     }
}
