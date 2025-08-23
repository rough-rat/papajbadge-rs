#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

#![no_std]
#![no_main]


use {ch58x_hal as hal};
use ch58x_hal::Peripheral;
use hal::delay::CycleDelay;
use hal::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use hal::{delay, isp, peripherals};

use hal::with_safe_access;
use ch58x_hal::pac::{PFIC, SYS};
use ch58x_hal::pac::Interrupt;
use qingke::riscv::asm::{wfi, nop};
// use core::ptr::{read_volatile, write_volatile};

use embedded_hal_local::delay::DelayNs;
use papajbadge_rs::helpers::delay_systick_ms;
// use hal::delay::CycleDelay;


use hal::rtc::{Rtc};

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};


use papajbadge_rs::helpers::{blinky, get_configured_rtc, enter_sleep};
use papajbadge_rs::audio::{get_char_for_t, chiptune_loop};
use papajbadge_rs::logger::init as init_logger;

use papajbadge_rs::log;

use hal::uart::UartTx;
static mut SERIAL: Option<UartTx<peripherals::UART0>> = None;


use hal::spi::{BitOrder, Spi};
use ch58x_hal::spi::Polarity;
use hal::prelude::*;

use spi_memory::prelude::*;
use spi_memory::series25::Flash;

#[no_mangle]
pub extern "C" fn RTC() {
    unsafe { Rtc.ack_timing(); }
}


#[embassy_executor::task]
async fn async_blink(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low, OutputDrive::_5mA);

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(150)).await;
        led.set_low();
        Timer::after(Duration::from_millis(150)).await;
    }
}

#[qingke_rt::entry]
unsafe /*you can just do this? */ fn main() -> ! {
    let config = hal::Config::default();
    let mut p = hal::init(config);

    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    // let mut but = Input::new(p.PB22, Pull::None);
    let serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!( "\n\n\nHello World!");

    unimplemented!("borrow hard");
    // p = flash_test(p); 
    blinky(Output::new(p.PA8, Level::Low, OutputDrive::_5mA));
    loop{unsafe{nop()}}; //not reachable but rust knows better
}


pub fn flash_test(p: hal::Peripherals){
    let mut spi_config = hal::spi::Config::default();
    spi_config.frequency = 20.MHz();
    spi_config.bit_order = BitOrder::MsbFirst;
    spi_config.clock_polarity = Polarity::IdleLow;
    let mut spi = Spi::new(p.SPI0, p.PA13, p.PA14,p.PA15, spi_config);

    let cs = Output::new(p.PA12, Level::High, OutputDrive::_5mA);

    let mut flash = Flash::init(spi, cs).unwrap();

    let id = flash.read_jedec_id().unwrap();
    log!("{:?}", id);

    let mut addr = 0;
    const BUF: usize = 32;
    let mut buf = [0; BUF];

    while addr < 1024 {
        flash.read(addr, &mut buf).unwrap_or_else(|e| {
            log!( "Error reading flash at {}: {:?}", addr, e);
            return;
        });
        log!( "{:?} ", &buf);

        addr += BUF as u32;
    }
}