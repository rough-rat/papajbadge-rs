use {ch58x_hal as hal};
use core::fmt::Write;
use ch58x_hal::{peripherals, with_safe_access};
use hal::gpio::{Output};
use embedded_hal_local::delay::DelayNs;
use qingke::riscv::asm::{nop, wfi};

use core::ptr::{read_volatile};
use hal::uart::UartTx;
use hal::delay::CycleDelay;
use hal::rtc::{DateTime, Rtc};
use core::sync::atomic::{AtomicBool, Ordering};
use ch58x_hal::pac::{self, PFIC, SYS}; // for SYSTICK

use crate::log;


// #[macro_export]
// macro_rules! log {
//     ($($arg:tt)*) => {
//         unsafe {
//             use core::fmt::Write;
//             use core::writeln;

//             if let Some(uart) = SERIAL.as_mut() {
//                 writeln!(uart, $($arg)*).unwrap();
//             }
//         }
//     }
// }

/* CAUTION: VIBECODED CRAP, TODO: rewrite */

#[allow(dead_code)]
static mut SERIAL: Option<UartTx<peripherals::UART0>> = None;

#[allow(dead_code)]
static SYSTICK_INIT: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
fn init_systick() {
    if SYSTICK_INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        // Enable SysTick counter in upcount mode sourced from HCLK.
        let st = unsafe { &*pac::SYSTICK::PTR }; // 0xE000_F000
        st.ctlr().modify(|_, w| {
            w.stclk().hclk()   // HCLK source
             .mode().upcount() // upcount
             .ste().set_bit()  // enable counter
        });
    }
}

#[allow(dead_code)]
#[inline]
pub fn delay_systick_cycles(cycles: u64) {
    init_systick();
    let st = unsafe { &*pac::SYSTICK::PTR };
    let start = st.cnt().read().bits();
    while st.cnt().read().bits().wrapping_sub(start) < cycles {
        core::hint::spin_loop();
    }
}

#[allow(dead_code)]
#[inline]
pub fn delay_systick_us(us: u32) {
    let hclk = hal::sysctl::clocks().hclk.to_Hz() as u64;
    let cycles = (hclk * us as u64 + 999_999) / 1_000_000; // round up
    delay_systick_cycles(cycles);
}

#[allow(dead_code)]
#[inline]
pub fn delay_systick_ms(ms: u32) {
    // Prevent overflow for very large ms.
    let mut remaining = ms;
    while remaining > 0 {
        let chunk = remaining.min(1000); // process in <=1s chunks
        delay_systick_us(chunk * 1000);
        remaining -= chunk;
    }
}

/* CAUTION END */


pub unsafe fn peek_register(ptr: u32){
    let dupa = read_volatile(ptr as *const u8); // read to ensure the write is complete
    log!("0x{:08X}: 0x{:02X}", ptr, dupa);
}

pub fn blinky(mut led: Output<'_, ch58x_hal::peripherals::PA8>){
    let mut delay = CycleDelay;
    loop{
        led.toggle();
        delay.delay_ms(100);
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


#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let pin = unsafe { hal::peripherals::PB7::steal() };
    let uart = unsafe { hal::peripherals::UART0::steal() };

    let mut serial = UartTx::new(uart, pin, Default::default()).unwrap();

    let _ = writeln!(&mut serial, "\n\n\n{}", info);

    loop {}
}


pub fn enter_sleep(){
    let sys = unsafe { &*SYS::PTR };
    let pfic = unsafe { &*PFIC::PTR };

    unsafe{/*
        * R16 POWER_PLAN Register (Reset: 0x03FB)
        * 
        * [1] Bit 15:    RB_PWR_PLAN_EN   - Sleep planning enable
        [0010] Bit 14-11: RB_PWR_MUST_001_0 - Reserved (0010b)  
        * [0] Bit 10:    RB_PWR_DCDC_PRE  - DC-DC bias enable
        * [0] Bit 9:     RB_PWR_DCDC_EN   - DC-DC enable (immediate)
        * [1] Bit 8:     RB_PWR_LDO_EN    - Internal LDO control

        * [1] Bit 7:     RB_PWR_SYS_EN    - System power control
        * [1] Bit 6:     Reserved (write 0)
        * [0] Bit 5:     Reserved 
        * [1] Bit 4:     RB_PWR_RAM30K    - RAM30K power supply

        * [1] Bit 3:     RB_PWR_EXTEND    - USB/RF config power
        * [1] Bit 2:     RB_PWR_CORE      - Core/peripherals power
        * [1] Bit 1:     RB_PWR_RAM2K     - RAM2K power supply  
        * [1] Bit 0:     RB_PWR_XROM      - FlashROM power supply
        *
        * All bits control sleep planning except bits 10&9 (immediate effect)
        */

        //must use direct bits for now, pwr_plan_en is set as read only in SVD
        with_safe_access(||{
            sys.power_plan().modify(|_, w| {
                w.bits(0b1001_0001_0001_0111)
            });
        });

        // for even deeper sleep, todo
        // isp::flash_rom_reset();
        // sys.flash_cfg().write(|w| {
        //     w.bits(0x04)
        // });

        with_safe_access(|| {
            sys.xt32m_tune().write(|w| {
                w.xt32m_i_bias().bits(0b11)
            });
        });


        with_safe_access(|| {
            sys.pll_config().modify(|r, w| 
                w.bits(r.bits() | (1 << 5)));
        });

        pfic.sctlr().modify(|_, w| {
            w.sleepdeep().bit(true)
        });
        
        wfi();
        nop();
        nop();

        with_safe_access(|| {
            sys.pll_config().modify(|r, w| 
                w.bits(r.bits() & !(1 << 5)));
        });
    }
}


// the default safe access wrapper sometimes doesn't work on debug run
// #[inline(never)]
// #[no_mangle]
// pub unsafe extern "C" fn enable_rwa() -> bool {
//     let sys = unsafe { &*SYS::PTR };

//     let reg_ptr = 0x4000_1040 as *mut u8;

//     write_volatile(reg_ptr, 0x57);
//     nop();
//     write_volatile(reg_ptr, 0xA8);
//     nop(); nop();

//     sys.safe_access_sig().read().safe_acc_act().bit_is_set()
// }