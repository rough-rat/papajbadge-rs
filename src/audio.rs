use {ch58x_hal as hal};

use embedded_hal_local::delay::DelayNs;
use hal::delay::CycleDelay;


pub fn get_char_for_t(t: i32) -> u8 {
    let s = b"36364689";
    let part1 = ((t * (s[(t >> 13 & 7) as usize] & 15) as i32) / 12 & 128) as u8;
    let part2 = ((((((t >> 12) ^ ((t >> 12) - 2)) % 11) * t) / 4 | (t >> 13)) & 127) as u8;
    part1 + part2
}

pub fn chiptune_loop() {
    
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

