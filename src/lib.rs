#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![no_std]


use {ch58x_hal as hal};
use ch58x_hal::peripherals;
use hal::uart::UartTx;
use hal::rtc::{DateTime, Rtc};

use ch58x_hal::ble::ffi::TMOS_SystemProcess;
use embassy_time::{Duration, Ticker};
use qingke_rt::highcode;

pub mod helpers;
pub mod audio;
pub mod logger;

// use crate::ble_periph;
pub mod ble_periph;

#[allow(unused)]
static mut SERIAL: Option<UartTx<peripherals::UART0>> = None;


#[highcode]
#[embassy_executor::task]
pub async fn tmos_mainloop() {
    let mut ticker = Ticker::every(Duration::from_micros(300));
    loop {
        ticker.next().await;
        unsafe {
            TMOS_SystemProcess();
        }
    }
}

pub fn get_configured_rtc() -> Rtc {
    let mut rtc = Rtc {};
    let now = rtc.now();

    if now.year < 2025 {
        log!("RTC not set, setting to a meaningless random date");
        rtc.set_datatime(
            DateTime { year: 2025, month: 4, day: 2,
                hour: 21,minute: 36, second: 0,
                millisecond: 0,
            }
        );
    } else {
        log!("RTC already set to: {:02}:{:02}:{:02} {:02}/{:02}/{}", 
            now.hour, now.minute, now.second, now.month, now.day, now.year);
    }
    rtc
}

#[no_mangle]
pub extern "C" fn RTC() {
    Rtc.ack_timing();
}

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let pin = unsafe { hal::peripherals::PB7::steal() };
    let uart = unsafe { hal::peripherals::UART0::steal() };

    let mut serial = UartTx::new(uart, pin, Default::default()).unwrap();

    let _ = writeln!(&mut serial, "\n\n\n{}", info);

    loop {}
}
