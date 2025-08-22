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

`wchisp flash <PATH>`. You must press "boot" button and reset power to enter ISP mode

### Enabling not-SWD interface

Supposedly this interface is not meant to be used for flashing. After unlocking
the device it works fine though. The workflow is much more convenient with debug
probe - with USB flashing, to load new firmware, one must disconnect and connect with "boot" button
pressed (hold reset -> hold boot -> release reset is not working either fo some 
reason)

The chip may need unlocking, using USB interface first.

Connect the device, and call `make mcu-unlock`

### flashing via not-SWD

After unprotecting the chip, `wlink -v flash <BINARY>`

### not-SWD + GDB workflow

In 1st shell, call `make spawn-openocd` - it must be running in the background.

It also claims the USB probe, so wlink will not work in parallel to it. 

You may need to kill openocd manually if something goes wrong.

-----

In 2nd shell, call either `make debug` or `make attach` (to skip loading new binary)

Refer to Makefile for more information.

### Flashing target put to sleep/wfi() mode

Reset the board with "boot" button pressed. This does not enter the bootloader
mode (like in esp32), but the firmware checks on start if the button is pressed
and enters blinky busyloop (so probe can attach to it with no issue).

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
            - [x] run (6mA @ 6mhz)
            - [x] idle (1.6mA, agrees with datasheet)
            - [x] halt (150-250uA, should be 320uA)
            - [ ] sleep
        - [x] check CR2032/USB power XORing (removed in rev2)
        - [x] check if LDO stable (removed in rev2)
    - [x] clocks
        - [x] check LSE (waveform visible without activating in code, probing glitches time counting without halting MCU)
        - [x] check HSE (probing glitches UART baudrate)
        - [x] check RTC (set/read works, counting works)
    - [ ] onboard flash
        - [x] chip ID + read
        - [ ] write
        - [ ] DMA
    - [ ] bluetooth
        - [x] run BLE connection (using examples from hal)
        - [x] run bluetooth scan/ADV reception
        - [ ] port bluetooth to this repo (embassy/hal dependency hell got me stuck)
- [ ] firmware
    - [ ] integrate embassy async
    - [ ] UART not working in debug builds (clock not settable in debug other than 6mhz)
    - [x] UART logging


## Debug build issues

TODO expand

```
debug build
0x40001000:	0x00000000	0x00000000	0x00140005	0x00200000 // 0x40001008 - clock register
0x40001010:	0x00000000	0x00000000	0x00000000	0x00000000 // PLL difference 0x48 vs 0x05
0x40001020:	0x092211df	0x00000200	0x00000000	0x82c31011 // conclusion - writing there doesnt work
0x40001030:	0x00000230	0x00000000	0x0083afab	0x00000000 // count register (irrelevant)
0x40001040:	0xcf0c8270	0x0010dd00	0x4a000094	0x00320000 // RB_SAFE_ACC_MODE 00 vs 11
0x40001050:	0x010000b2	0x00000000	0x0000a00f	0x00000000 // wdog (irrelevant)

--release
0x40001000:	0x00000000	0x00000000	0x00140048	0x00200000
0x40001010:	0x00000000	0x00000000	0x00000000	0x00000000
0x40001020:	0x092211df	0x00000200	0x00000000	0x82c31011
0x40001030:	0x00000230	0x00000000	0x01748622	0x00000000
0x40001040:	0xee0c8240	0x0010dd00	0xca000094	0x00320000
0x40001050:	0x010000b2	0x00000000	0x0000a00f	0x00000000
```