#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(generic_const_exprs)]
#![no_std]
#![no_main]

use ch58x_hal as hal;
use embassy_executor::Spawner;
use hal::gpio::{Level, Output, OutputDrive};
use hal::peripherals;
use hal::uart::UartTx;
use hal::ble;
use papajbadge_rs::logger::init as init_logger;
use papajbadge_rs::{get_configured_rtc, log, tmos_mainloop};
use papajbadge_rs::sr_led_driver::SrLedDriver;

use papajbadge_rs::ble_periph::{common_init, devinfo_init, peripheral};
use papajbadge_rs::ble_periph::current_time_service::current_time_init;

use embassy_time::Duration;
use embassy_time::Timer;
use ch58x_hal::gpio::AnyPin;

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

#[embassy_executor::task]
async fn sr_driver_task(mut sr_driver: SrLedDriver<'static, peripherals::SPI0, 48>) -> ! {
    log!("Shift register driver task started");
    sr_driver.enable_output();

    let mut lfsr: u32 = 0xACE1_1234;
    let mut frame = [0u8; 48];
    loop {
        for (i, byte) in frame.iter_mut().enumerate() {
            // Simple xorshift32 PRNG for repeatable-yet-varied patterns.
            lfsr ^= lfsr << 13;
            lfsr ^= lfsr >> 17;
            lfsr ^= lfsr << 5;
            let val = lfsr as u8 ^ (i as u8);
            if val > 128 {
                *byte = 0xFF;
            } else {
                *byte = 0x00;
            }
        }

        sr_driver.write(0, &frame).unwrap();
        sr_driver.update();
        sr_driver.enable_output();
        Timer::after(Duration::from_millis(500)).await;
        sr_driver.disable_output();
    }
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = hal::Config::default();
    config.clock.use_pll_60mhz().enable_lse();
    let p = hal::init(config);
    hal::embassy::init();

    let mut ena = Output::new(p.PB13, Level::Low, OutputDrive::_5mA);
    ena.set_high();

    let serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    init_logger(serial);
    log!("\n\n\nArtsyWatch Firmware Started!");

    // Shift register control pins for LED matrix column driving
    // SCK - PA13 (SPI Clock)
    // DAT - PA14 (SPI MOSI/Data)
    // OE  - PA8  (Output Enable, directly GPIO-controlled)
    // LAT - PA9  (Latch, directly GPIO-controlled)

    // Initialize shift register LED driver from pins
    let mut sr_driver = 
        SrLedDriver::<_, 48>::new_from_pins::<false, _, _, _, _>
        (
            p.SPI0, p.PA13, p.PA14, p.PA8, p.PA9
    );

    log!("Shift register driver initialized");
    log!("  SPI: SCK=PA13, DAT=PA14");
    log!("  GPIO: OE=PA8, LAT=PA9");

    // Quick connectivity check before starting the main task.
    sr_driver.test_write(0xFF);
    sr_driver.test_write(0x00);
    log!("Starting shift register background task...");
    spawner.spawn(sr_driver_task(sr_driver)).unwrap();

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
        current_time_init();
    }

    // Main_Circulation
    spawner.spawn(tmos_mainloop()).unwrap();

    // Application code
    peripheral(spawner, task_id, sub, rtc).await
}
