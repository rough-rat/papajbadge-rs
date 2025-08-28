#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![no_std]
#![no_main]

use ch58x_hal as hal;
use embassy_executor::Spawner;
use hal::gpio::{Level, Output, OutputDrive, Pin};
use hal::uart::UartTx;
use hal::{ble};
use papajbadge_rs::logger::init as init_logger;
use papajbadge_rs::{get_configured_rtc, log, tmos_mainloop};

use papajbadge_rs::ble_periph::{common_init, devinfo_init, peripheral};
use papajbadge_rs::ble_periph::blinky_service::{blinky_init, blinky_service_loop};
use papajbadge_rs::ble_periph::current_time_service::current_time_init;
use papajbadge_rs::ble_periph::hid_service::hid_init; // added

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = hal::Config::default();
    config.clock.use_pll_60mhz().enable_lse();
    let p = hal::init(config);
    hal::embassy::init();

    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    let serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!("\n\n\nHello World!");

    spawner.spawn(blinky_service_loop(p.PA8.degrade())).unwrap();

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

    unsafe {
        common_init();
        devinfo_init();
        hid_init(); // added
        blinky_init();
        current_time_init();
    }

    // Main_Circulation
    spawner.spawn(tmos_mainloop()).unwrap();

    // Application code
    peripheral(spawner, task_id, sub).await
}
