#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

#![no_std]
#![no_main]


use {ch58x_hal as hal};
use hal::delay::CycleDelay;
use hal::gpio::{ Input, Level, Output, OutputDrive, Pull};

use ch58x_hal::pac::Interrupt;
use papajbadge_rs::helpers::enable_sleep;

use embedded_hal_local::delay::DelayNs;
use hal::rtc::{Rtc};

use papajbadge_rs::{get_configured_rtc, helpers};
use papajbadge_rs::log;
use helpers::{blinky, enter_sleep};

use papajbadge_rs::logger::init as init_logger;

use hal::uart::UartTx;

#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default(); 
    config.low_power = true; //800uA->150uA
    let p = hal::init(config);

    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    let but = Input::new(p.PB22, Pull::None);

    let serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!( "\n\n\nHello World!");

    let mut rtc = get_configured_rtc();

    if but.is_low() {
        // without that, the board is impossible to reprogram without power cycling
        log!("Button pressed, loopin' time\n");
        let led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
        blinky(led);
    } else {
        let mut led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
        led.set_high();

        rtc.enable_timing(hal::rtc::TimingMode::_0_5S);
        rtc.ack_timing();

        unsafe {
            qingke::pfic::enable_interrupt(Interrupt::RTC as u8);
        }
        enable_sleep();

        rtc_loop(rtc, led);
    }
}


fn rtc_loop(rtc: Rtc, mut led: Output<'_, ch58x_hal::peripherals::PA8>) -> ! {
    let mut counter: u32 = 0;
    let mut delay = CycleDelay;

    loop{
        led.toggle();

        enter_sleep();       
        delay.delay_us(1000);
        let now = rtc.now();
        log!("T{:02}:{:02}:{:02}, loop {}\n", 
            now.hour, now.minute, now.second, counter);
        counter += 1;
        delay.delay_us(500);
    }
}