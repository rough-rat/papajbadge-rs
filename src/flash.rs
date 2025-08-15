use {ch58x_hal as hal};

use hal::spi::{BitOrder, Spi};
use ch58x_hal::spi::Polarity;
use hal::prelude::*;
use hal::uart::UartTx;
use hal::gpio::{Level, Output, OutputDrive};

use spi_memory::prelude::*;
use spi_memory::series25::Flash;

use core::writeln;
use core::fmt::Write;
pub fn flash_test(mut serial: UartTx<'_, ch58x_hal::peripherals::UART0>, p: hal::Peripherals){
    let mut spi_config = hal::spi::Config::default();
    spi_config.frequency = 20.MHz();
    spi_config.bit_order = BitOrder::MsbFirst;
    spi_config.clock_polarity = Polarity::IdleLow;
    let mut spi = Spi::new(p.SPI0, p.PA13, p.PA14,p.PA15, spi_config);

    let cs = Output::new(p.PA12, Level::High, OutputDrive::_5mA);

    let mut flash = Flash::init(spi, cs).unwrap();

    let id = flash.read_jedec_id().unwrap();
    writeln!(serial,"{:?}", id).ok();

    let mut addr = 0;
    const BUF: usize = 32;
    let mut buf = [0; BUF];

    while addr < 1024 {
        flash.read(addr, &mut buf).unwrap_or_else(|e| {
            writeln!(serial, "Error reading flash at {}: {:?}", addr, e).ok();
            return;
        });
        writeln!(serial, "{:?} ", &buf).ok();

        addr += BUF as u32;
    }
}