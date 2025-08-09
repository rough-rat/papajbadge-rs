#![no_std]
#![no_main]

use embedded_hal_local::delay::DelayNs;
use hal::delay::CycleDelay;
use hal::gpio::{Level, Output, OutputDrive};
use qingke::riscv::asm::wfi;
use {ch58x_hal as hal};

use hal::uart::UartTx;
use core::writeln;
use core::fmt::Write;

// use hal::spi::{BitOrder, Spi};
// use ch58x_hal::spi::Polarity;
// use hal::prelude::*;

// use spi_memory::prelude::*;
// use spi_memory::series25::Flash;

use hal::rtc::{DateTime, Rtc};
use ch58x_hal::pac::{PFIC, SYS};


fn get_char_for_t(t: i32) -> u8 {
    let s = b"36364689";
    let part1 = ((t * (s[(t >> 13 & 7) as usize] & 15) as i32) / 12 & 128) as u8;
    let part2 = ((((((t >> 12) ^ ((t >> 12) - 2)) % 11) * t) / 4 | (t >> 13)) & 127) as u8;
    part1 + part2
}

#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default();
    config.clock.use_pll_60mhz().enable_lse();

    // config.clock.use_lse_32k().enable_lse(); //not needed? why?


    let p = hal::init(config);

    let mut delay = CycleDelay;

    // LED PA8
    // let mut led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
    let mut spk = Output::new(p.PA9, Level::High, OutputDrive::_20mA);
    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_low();

    let mut serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    let _ = serial.blocking_flush();

    writeln!(serial, "\n\n\nHello World!").unwrap();


    // let mut spi_config = hal::spi::Config::default();
    // spi_config.frequency = 20.MHz();
    // spi_config.bit_order = BitOrder::MsbFirst;
    // spi_config.clock_polarity = Polarity::IdleLow;
    // let mut spi = Spi::new(p.SPI0, p.PA13, p.PA14,p.PA15, spi_config);

    // let cs = Output::new(p.PA12, Level::High, OutputDrive::_5mA);

    // let mut flash = Flash::init(spi, cs).unwrap();

    // let id = flash.read_jedec_id().unwrap();
    // writeln!(serial,"{:?}", id).ok();

    // let mut addr = 0;
    // const BUF: usize = 32;
    // let mut buf = [0; BUF];

    // while addr < 1024 {
    //     flash.read(addr, &mut buf).unwrap_or_else(|e| {
    //         writeln!(serial, "Error reading flash at {}: {:?}", addr, e).ok();
    //         return;
    //     });
    //     writeln!(serial, "{:?} ", &buf).ok();

    //     addr += BUF as u32;
    // }

    let mut rtc = Rtc {};

    let now = rtc.now();

    if now.year < 2025 {
        writeln!(serial, "RTC not set, setting to a meaningless random date").unwrap();
        rtc.set_datatime(
            DateTime {
                year: 2025,
                month: 4,
                day: 2,
                hour: 21,
                minute: 36,
                second: 0,
                millisecond: 0,
            }
        );
    } else {
        writeln!(serial, "RTC already set to: {:02}:{:02}:{:02} {:02}/{:02}/{}", 
            now.hour, now.minute, now.second, now.month, now.day, now.year).unwrap();
    }

    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };

    sys.slp_wake_ctrl().modify(|_, w| {
        w.slp_rtc_wake().bit(true)
    });

    unsafe{
        sys.rtc_mode_ctrl().modify(|_, w| {
            w.rtc_tmr_mode().bits(0b011).rtc_tmr_en().bit(true)
        });
    }
    // pfic.sctlr().modify(|_, w| {
    //     w.sleepdeep().bit(true)
    // });

    let mut counter: u32 = 0;
    loop{
        // sys.power_plan().modify(|_, w| {
        //     w.pwr_sys_en().bit(false)
        // });

        unsafe{
            wfi();
            sys.rtc_flag_ctrl().modify(|_, w| {
                w.rtc_tmr_clr().bit(true)
            });
        }
       
        let now = rtc.now();
        writeln!(serial, "T{:02}:{:02}:{:02}, loop {}", 
            now.hour, now.minute, now.second, counter).unwrap();
        counter += 1;
    }


    let mut period: u32 = 20;

    unsafe {
        let tmr = ch58x_hal::pac::TMR0::steal();
        tmr.ctrl_mod().write(|w| w.tmr_all_clear().bit(true));
        tmr.cnt_end().write(|w| w.cnt_end().bits(0x100));
        tmr.ctrl_mod().write(|w| w.tmr_all_clear().bit(true));
        tmr.ctrl_mod().write(|w| 
            w.tmr_all_clear().bit(false)
            .tmr_mode_in().bit(false)
            .tmr_count_en().bit(true)
            .tmr_out_polar__rb_tmr_cap_count().bit(true)
            .tmr_pwm_repeat__rb_tmr_cap_edge().bits(0b11)
            .tmr_out_en().bit(true)
        );
        tmr.fifo().write(|w|  w.fifo().bits(0x08) );

        loop {
            delay.delay_us(1);
            period += 1;

            if period % 1000_00 == 0 {
                let now = rtc.now();
                 writeln!(serial, "Current RTC time: {:02}:{:02}:{:02} {:02}/{:02}/{}", 
                    now.hour, now.minute, now.second, now.month, now.day, now.year).unwrap();
            }

            // let v = get_char_for_t(period as i32);
            // tmr.fifo().write(|w|  w.fifo().bits(v as u32) );
        }
    }
}


#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let pin = unsafe { hal::peripherals::PB7::steal() };
    let uart = unsafe { hal::peripherals::UART0::steal() };

    let mut serial = UartTx::new(uart, pin, Default::default()).unwrap();

    let _ = writeln!(&mut serial, "\n\n\n{}", info);

    loop {}
}