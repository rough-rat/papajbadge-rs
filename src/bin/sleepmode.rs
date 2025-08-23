#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

#![no_std]
#![no_main]


use {ch58x_hal as hal};
use hal::delay::CycleDelay;
use hal::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use hal::{delay, isp, peripherals};

use hal::with_safe_access;
use ch58x_hal::pac::{PFIC, SYS};
use ch58x_hal::pac::Interrupt;
use qingke::riscv::asm::{wfi, nop};
// use core::ptr::{read_volatile, write_volatile};

use embedded_hal_local::delay::DelayNs;
use hal::rtc::{Rtc};

use papajbadge_rs::helpers;
use papajbadge_rs::log;
use helpers::{blinky, get_configured_rtc, enter_sleep};

use papajbadge_rs::logger::init as init_logger;

use hal::uart::UartTx;
static mut SERIAL: Option<UartTx<peripherals::UART0>> = None;

#[no_mangle]
pub extern "C" fn RTC() {
    unsafe { Rtc.ack_timing(); }
}
#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default(); 
    config.low_power = true; //800uA->150uA
    let p = hal::init(config);

    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    let mut but = Input::new(p.PB22, Pull::None);

    let mut serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!( "\n\n\nHello World!");
    if but.is_low() {
        log!("Button pressed, loopin' time\n");
        let mut rtc = get_configured_rtc(); 
        let led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
        blinky(led);
    } else {
        let mut led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
        led.set_high();

        let mut rtc = get_configured_rtc(); 
        rtc.enable_timing(hal::rtc::TimingMode::_0_5S);
        rtc.ack_timing();

        unsafe {
            qingke::pfic::enable_interrupt(Interrupt::RTC as u8);
        }
        enable_sleep();

        rtc_loop(rtc, led);
    }
   
    loop{unsafe{nop()}}; //not reachable but rust knows better
}


fn rtc_loop(mut rtc: Rtc, mut led: Output<'_, ch58x_hal::peripherals::PA8>) -> ! {
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

fn enable_sleep(){

    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };
     unsafe{
        with_safe_access(||{
            // wakeup from RTC ISR, memory stays active (?)
            sys.slp_wake_ctrl().modify(|_, w| {
                w.slp_rtc_wake().bit(true).wake_ev_mode().bit(false)
            });
        });
        with_safe_access(||{
            // XXX cycles of delay
            sys.slp_power_ctrl().modify(|_, w| {
                w.wake_dly_mod().bits(0b00)
            });
        });

    }
}