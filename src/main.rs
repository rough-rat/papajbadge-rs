#![no_std]
#![no_main]

use ch58x_hal::delay;
use embedded_hal_local::delay::DelayNs;
use hal::delay::CycleDelay;
use hal::gpio::{Level, Output, OutputDrive};
use hal::with_safe_access;
use qingke::riscv::asm::{wfi, nop};
use {ch58x_hal as hal};

use hal::uart::UartTx;
use core::ptr::{read_volatile, write_volatile};
use core::writeln;
use core::fmt::Write;


use hal::rtc::{DateTime, Rtc};
use ch58x_hal::pac::{PFIC, SYS};

mod flash;
use flash::flash_test;


#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default();
    config.clock.use_pll_60mhz().enable_lse();
    // config.clock.use_lse_32k().enable_lse(); //not needed? why?

    let p = hal::init(config);

    // let mut spk = Output::new(p.PA9, Level::High, OutputDrive::_20mA);    
    let mut led = Output::new(p.PA8, Level::Low, OutputDrive::_5mA);
    let mut ena = Output::new(p.PA4, Level::Low, OutputDrive::_5mA);
    ena.set_high();


    let mut serial = UartTx::new(p.UART0, p.PB7, Default::default()).unwrap();
    let _ = serial.blocking_flush();

    writeln!(serial, "\n\n\nHello World!").unwrap();
 
    let mut delay = CycleDelay;
    // let mut rtc = Rtc {};

    let (mut rtc, mut serial) = get_configured_rtc(serial); 

    // p = flash_test(serial, p);
    // blinky(led);
    // enable_shit(serial);
    // rwa_method2();
    rtc_loop(serial, rtc);
   
    // chiptune_loop();
    loop{unsafe{nop()}};

}

unsafe fn peek_register(ptr: u32, serial: &mut UartTx<'_, ch58x_hal::peripherals::UART0>){
    let dupa = read_volatile(ptr as *const u16); // read to ensure the write is complete
    writeln!(serial, "0x{:08X}: 0x{:04X}", ptr, dupa).unwrap();
}

fn blinky(mut led: Output<'_, ch58x_hal::peripherals::PA8>){
    let mut delay = CycleDelay;

    loop{
        led.toggle();
        delay.delay_ms(10);
    }
}

fn enable_shit(mut serial: UartTx<'_, ch58x_hal::peripherals::UART0>){

    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };

    unsafe{
        let reg_ptr = 0x4000_1040 as *mut u8;

        // write_volatile(reg_ptr, 0x57);
        // nop();
        // write_volatile(reg_ptr, 0xA8);
        // nop(); nop();

        with_safe_access(||{            

            sys.rtc_mode_ctrl().modify(|_, w| {
                w.rtc_tmr_mode().bits(0b011).rtc_tmr_en().bit(true)
            });

            
        // write_volatile(0x4000100E as *mut u8, 0x28); // enable RWA
        // write_volatile(reg_ptr, 0x00);
        });

    }
}



#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn enable_rwa() -> bool {
    let reg_ptr = 0x4000_1040 as *mut u8;

    write_volatile(reg_ptr, 0x57);
    nop();
    write_volatile(reg_ptr, 0xA8);
    // *reg_ptr = 0x57;
    // *reg_ptr = 0xA8;

    // sys.safe_access_sig().write(|w| {
    //     // enable RTC timer wakeup
    //     w.safe_access_sig().bits(0x57)
    // });

    // sys.safe_access_sig().write(|w| {
    //     // enable RTC timer wakeup
    //     w.safe_access_sig().bits(0xA8)
    // });
    nop(); nop();
    write_volatile(0x4000100E as *mut u8, 0x28); // enable RWA
    write_volatile(reg_ptr, 0x00);

    let dupa = read_volatile(0x4000100E as *const u8); // read to ensure the write is complete


    if dupa != 0x28 {
        return false; // failed to enable RWA
    }
    let sys = unsafe { &*SYS::PTR };
    sys.safe_access_sig().read().safe_acc_act().bit_is_set()
}

fn get_configured_rtc(mut serial: UartTx<'_, ch58x_hal::peripherals::UART0>) -> (Rtc, UartTx<'_, ch58x_hal::peripherals::UART0>) {
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
    (rtc, serial)
}

fn rwa_method2() {

    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };
     unsafe{
        // let is_safe = enable_rwa();
        // if !is_safe {
        //     writeln!(serial, "Failed to enable RWA, exiting...").unwrap();
        // }
        with_safe_access(||{
            // wakeup from RTC ISR, memory stays active (?)
            sys.slp_wake_ctrl().modify(|_, w| {
                w.slp_rtc_wake().bit(true).wake_ev_mode().bit(true)
            });
        });
        with_safe_access(||{
            // 520 cycles of delay
            sys.slp_power_ctrl().modify(|_, w| {
                w.wake_dly_mod().bits(0b11)
            });
        });
        with_safe_access(||{
            // enable RTC timer every 1s
            sys.rtc_mode_ctrl().modify(|_, w| {
                w.rtc_tmr_mode().bits(0b011).rtc_tmr_en().bit(true)
            });
        });
    }
    // u want deepsleep?
    pfic.sctlr().modify(|_, w| {
        w.sleepdeep().bit(false)
    });
}

fn rtc_loop(mut serial: UartTx<'_, ch58x_hal::peripherals::UART0>, rtc: Rtc) {

    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };
    let mut counter: u32 = 0;
    let mut delay = CycleDelay;

    loop{     
        unsafe{
            // enable_rwa();
            // write 0 to enable, should sleep on next WFI
            peek_register(0x40001020, &mut serial);

            with_safe_access(||{
                sys.power_plan().modify(|_, w| {
                    w.pwr_sys_en().bit(false)
                });
            });
            peek_register(0x40001020, &mut serial);
            delay.delay_ms(1000);

            wfi();
            // clear RTC timer flag
            sys.rtc_flag_ctrl().modify(|_, w| {
                w.rtc_tmr_clr().bit(true)
            });
        }
       
        let now = rtc.now();
        writeln!(serial, "T{:02}:{:02}:{:02}, loop {}", 
            now.hour, now.minute, now.second, counter).unwrap();
        counter += 1;
    }
}


fn get_char_for_t(t: i32) -> u8 {
    let s = b"36364689";
    let part1 = ((t * (s[(t >> 13 & 7) as usize] & 15) as i32) / 12 & 128) as u8;
    let part2 = ((((((t >> 12) ^ ((t >> 12) - 2)) % 11) * t) / 4 | (t >> 13)) & 127) as u8;
    part1 + part2
}

fn chiptune_loop() {
    
    let mut period: u32 = 20;
    let mut delay = CycleDelay;

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

            let v = get_char_for_t(period as i32);
            tmr.fifo().write(|w|  w.fifo().bits(v as u32) );
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