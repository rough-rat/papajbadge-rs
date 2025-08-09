# papajbadge-rs

This repo contains firmware for papajbadge

It contains some nessesary mess:
 * pre-compiled openocd taken from mounriver's release (no-license)
 * vendored libs for bluetooth
 * ch58x pac - i had to patch TIM0 peripheral so it may be used for PWM output

## Requirements

You need WCH-LinkE probe - besides flashing/debugging via i-can't-belive-it's-not-SWD 
dual wire interface, you may also use 3V3 for power and RX for logs via UART.

Do not use WCH-Link, it nearly works, but contains some bugs, and does not work.

## The workflow

### Flashing as the WCH intended (via USB)

`wchisp flash <PATH>`

### Enabling not-SWD interface

Supposedly this interface is not meant to be used for flashing. After unlocking
the device it works fine though. The workflow is much more convenient with debug
probe - with USB flashing, to load new firmware, one must disconnect and connect with "boot" button
pressed (hold reset -> hold boot -> release reset is not working either fo some 
reason)

Sadly, the chip must be unlocked using USB interface first.

Connect the device, and call `make unlock-target`

### flashing via not-SWD

After unprotecting the chip, `wlink -v flash <BINARY>`

### not-SWD + GDB workflow

In 1st shell, call `make spawn-openocd` - it must be running in the background.

It also claims the USB probe, so wlink will not work in parallel to it. 

You may need to kill openocd manually if something goes wrong.

-----

In 2nd shell, call either `make debug` or `make attach` (to skip loading new binary)

Refer to Makefile for more information.

### More info

Currently I'm using makefiles, but in the end, i'd like to keep the environment
rust-centric.

You may also customize yout workflow with .cargo/config.toml.


## Progress

- [x] environment
    - [x] extract required files from https://github.com/ch32-rs/ch58x-hal to this repo
    - [x] fix TIM0 SVD description
    - [x] prepare gdb environment
    - [x] Logs via UART
- [ ] board bringup
    - [x] check if bluetooth works at all
    - [x] run PoC PWM output
    - [ ] PWM + DMA for audio output (TIM0 DOES NOT WORK WITH DMA)
    - [ ] speaker (current revision is fubar)
    - [ ] power
        - [ ] check power consumption + sleep modes
        - [ ] check CR2032/USB power XORing
        - [ ] check if LDO stable
    - [x] clocks
        - [x] check LSE (waveform visible without activating in code, probing glitches time counting without halting MCU)
        - [x] check HSE (probing glitches UART baudrate)
        - [x] check RTC (set/read works, counting works)
    - [ ] onboard flash
        - [x] chip ID + read
        - [ ] write
        - [ ] DMA
- [ ] firmware
    - [ ] integrate embassy async
    - [ ] UART not working in debug builds
    - [ ] UART logging
    - [ ] run bluetooth scan/ADV reception
    - [ ] run BLE connection