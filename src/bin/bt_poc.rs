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
use core::{ptr, slice};
use qingke_rt::highcode;


use embedded_hal_local::delay::DelayNs;
use papajbadge_rs::helpers::delay_systick_ms;
// use hal::delay::CycleDelay;


use hal::ble::ffi::*;
use hal::ble::gap::*;
use hal::ble::gatt::*;
use hal::ble::gattservapp::*;
use hal::ble::{gatt_uuid, TmosEvent};
use hal::{ble};



use hal::rtc::{Rtc};


use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker, Timer};


// mod flash;
// use flash::flash_test;

use papajbadge_rs::helpers::{blinky, get_configured_rtc, enter_sleep};
use papajbadge_rs::audio::{get_char_for_t, chiptune_loop};
use papajbadge_rs::logger::init as init_logger;

use papajbadge_rs::log;
use papajbadge_rs::ble_periph::*;

use hal::uart::UartTx;
static mut SERIAL: Option<UartTx<peripherals::UART0>> = None;


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

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    use hal::ble::ffi::*;

    let mut config = hal::Config::default();
    config.clock.use_pll_60mhz().enable_lse();
    let p = hal::init(config);
    hal::embassy::init();

    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    let serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!( "\n\n\nHello World!");

    spawner.spawn(blink(p.PA8.degrade())).unwrap();

    let rtc = get_configured_rtc();

    log!("System Clocks: {}", hal::sysctl::clocks().hclk);
    log!("ChipID: 0x{:02x}", hal::signature::get_chip_id());
    log!("RTC datetime: {}", rtc.now());

    log!("BLE Lib Version: {}", ble::lib_version());

    let mut ble_config = ble::Config::default();
    ble_config.pa_config = None;
    ble_config.mac_addr = [0x21, 0x37, 0x04, 0x20, 0x69, 0x96].into();
    let (task_id, sub) = hal::ble::init(ble_config).unwrap();
    log!("BLE hal task id: {}", task_id);

    let _ = GAPRole::peripheral_init().unwrap();

    unsafe {
        common_init();
        devinfo_init();
        blinky_init();
    }


    // Main_Circulation
    spawner.spawn(tmos_mainloop()).unwrap();

    // Application code
    peripheral(spawner, task_id, sub).await
}


#[highcode]
#[embassy_executor::task]
async fn tmos_mainloop() {
    let mut ticker = Ticker::every(Duration::from_micros(300));
    loop {
        ticker.next().await;
        unsafe {
            TMOS_SystemProcess();
        }
    }
}
