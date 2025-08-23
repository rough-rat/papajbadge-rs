#![allow(static_mut_refs)]

use {ch58x_hal as hal};
use core::{ptr, slice};

use hal::ble::ffi::*;
use hal::ble::gap::*;
use hal::ble::gatt::*;
use hal::ble::gattservapp::*;
use hal::ble::{gatt_uuid, TmosEvent};
use hal::{ble};

use crate::log;

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker, Timer};
use core::sync::atomic::{AtomicBool, Ordering};
use hal::gpio::{AnyPin, Level, Output, OutputDrive };

pub mod blinky_service;
pub mod current_time_service;

use blinky_service::{BLINKY_SERV_UUID, BLINKY_CLIENT_CHARCFG, blinky_notification};

const GAP_PAPAJ: u16 = 0x049F;

//TODO Current Time (0x2A2B)
// https://www.bluetooth.com/specifications/specs/object-push-profile-1-2/   ?
// Battery Level (0x2A19)
// Firmware Revision (0x2A26) 
// writable owner name
// Mute (0x2BC3)
// Object Push Profile (vcards)

const fn lo_u16(x: u16) -> u8 {
    (x & 0xff) as u8
}
const fn hi_u16(x: u16) -> u8 {
    (x >> 8) as u8
}

// GAP - SCAN RSP data (max size = 31 bytes)
static mut SCAN_RSP_DATA: &[u8] = &[
    // complete name
    0x12, // length of this data
    GAP_ADTYPE_LOCAL_NAME_COMPLETE,
    b'P',
    b'a',
    b'p',
    b'a',
    b'j',
    b'b',
    b'a',
    b'd',
    b'g',
    b'e',
    b'-',
    b'2',
    b'1',
    b'3',
    b'6',
    b'.',
    b'9',
    // Connection interval range
    0x05,
    GAP_ADTYPE_SLAVE_CONN_INTERVAL_RANGE,
    lo_u16(80), // units of 1.25ms, 80=100ms
    hi_u16(80),
    lo_u16(800), // units of 1.25ms, 800=1000ms
    hi_u16(800),
    // Tx power level
    0x02, // length of this data
    GAP_ADTYPE_POWER_LEVEL,
    0, // 0dBm
];

// const SIMPLEPROFILE_SERV_UUID: u16 = 0xFFE0;

// GAP - Advertisement data (max size = 31 bytes, though this is
// best kept short to conserve power while advertisting)
#[rustfmt::skip]
static mut ADVERT_DATA: &[u8] = &[
    0x02, // length of this data
    GAP_ADTYPE_FLAGS,
    GAP_ADTYPE_FLAGS_BREDR_NOT_SUPPORTED,
    // https://www.bluetooth.com/specifications/assigned-numbers/
    0x04,                             // length of this data including the data type byte
    GAP_ADTYPE_MANUFACTURER_SPECIFIC, // manufacturer specific advertisement data type
    lo_u16(GAP_PAPAJ),                // 0x07D7, Nanjing Qinheng Microelectronics Co., Ltd.
    hi_u16(GAP_PAPAJ),
    0x01, // remains manufacturer specific data

    // advertised service
    // 0x03,                  // length of this data
    // GAP_ADTYPE_16BIT_MORE, // some of the UUID's, but not all
    // lo_u16(BLINKY_SERV_UUID),
    // hi_u16(BLINKY_SERV_UUID),

    // advertise Current Time Service 0x1805
    0x03,
    GAP_ADTYPE_16BIT_MORE,
    lo_u16(gatt_uuid::CURRENT_TIME_UUID),
    hi_u16(gatt_uuid::CURRENT_TIME_UUID),
];

// GAP GATT Attributes
// len = 21 GAP_DEVICE_NAME_LEN
// max_len = 248
static ATT_DEVICE_NAME: &[u8] = b"papajbadge gatt stuff";
    // let uid = hal::isp::get_unique_id();

// System ID characteristic
const DEVINFO_SYSTEM_ID_LEN: usize = 8;

pub static mut SYSTEM_ID: [u8; 8] = [0u8; 8];
// The list must start with a Service attribute followed by
// all attributes associated with this Service attribute.
// Must use static mut fixed sized array, as it will be changed by Service to assign handles.
static mut DEVICE_INFO_TABLE: [GattAttribute; 7] =
    [
        // Device Information Service
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: unsafe { gatt_uuid::primaryServiceUUID.as_ptr() },
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            // The first must be a Service attribute
            value: &GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: &gatt_uuid::DEVINFO_SERV_UUID as *const _ as _,
            } as *const _ as _,
        },
        // System ID Declaration
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: unsafe { gatt_uuid::characterUUID.as_ptr() },
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: &GATT_PROP_READ as *const _ as _,
        },
        // System ID Value
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: &gatt_uuid::SYSTEM_ID_UUID as *const _ as _,
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: unsafe { SYSTEM_ID.as_ptr() },
        },
        // Serial Number String Declaration
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: unsafe { gatt_uuid::characterUUID.as_ptr() },
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: &GATT_PROP_READ as *const _ as _,
        },
        // Serial Number Value
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: &gatt_uuid::SERIAL_NUMBER_UUID as *const _ as _,
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: ptr::null(),
        },
        // Temperature
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: unsafe { gatt_uuid::characterUUID.as_ptr() },
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: &GATT_PROP_READ as *const _ as _,
        },
        // Serial Number Value
        GattAttribute {
            type_: GattAttrType {
                len: ATT_BT_UUID_SIZE,
                uuid: &gatt_uuid::TEMP_UUID as *const _ as _,
            },
            permissions: GATT_PERMIT_READ,
            handle: 0,
            value: ptr::null(),
        },
    ];

#[inline]
pub unsafe fn devinfo_init() {
    // DevInfo_AddService
    unsafe {
        unsafe extern "C" fn dev_info_on_read_attr(
            _conn_handle: u16,
            attr: *mut GattAttribute,
            value: *mut u8,
            plen: *mut u16,
            _offset: u16,
            max_len: u16,
            _method: u8,
        ) -> u8 {
            let raw_uuid = slice::from_raw_parts((*attr).type_.uuid, 2);
            let uuid = u16::from_le_bytes([raw_uuid[0], raw_uuid[1]]);
            log!("! on_read_attr UUID: 0x{:04x}", uuid);

            match uuid {
                gatt_uuid::SYSTEM_ID_UUID => {
                    *plen = DEVINFO_SYSTEM_ID_LEN as _;
                    ptr::copy(SYSTEM_ID.as_ptr(), value, DEVINFO_SYSTEM_ID_LEN);
                }
                gatt_uuid::SERIAL_NUMBER_UUID => {
                    // let out = hal::isp::get_unique_id();
                    // let out = [0xDE, 0xAD, 0xBE, 0xEF, 0x95, 0x27, 0x00, 0x00];
                    // TODO id to string
                    let out = b"No. 9527";
                    *plen = out.len() as _;
                    *plen = 8;
                    core::ptr::copy(out.as_ptr(), value, out.len());
                }
                gatt_uuid::TEMP_UUID => {
                    log!("temp uuid {:04x} {:p} {}", uuid, value, max_len);
                    let val: i16 = 32_00; // 0.01 degC
                    *plen = size_of_val(&val) as _;
                    core::ptr::copy(&val as *const _ as _, value, *plen as _);
                }
                _ => {
                    return ATT_ERR_ATTR_NOT_FOUND;
                }
            }

            return 0;
        }
        static DEV_INFO_SERVICE_CB: gattServiceCBs_t = gattServiceCBs_t {
            pfnReadAttrCB: Some(dev_info_on_read_attr),
            pfnWriteAttrCB: None,
            pfnAuthorizeAttrCB: None,
        };
        // DevInfo_AddService(); // Device Information Service
        // might fail, must check
        GATTServApp::register_service(
            &mut DEVICE_INFO_TABLE[..],
            GATT_MAX_ENCRYPT_KEY_SIZE,
            &DEV_INFO_SERVICE_CB,
        )
        .unwrap();
    }
}

/// GAP Role init
pub unsafe fn common_init() {
    let _ = GAPRole::peripheral_init().unwrap();

    // Setup the GAP Peripheral Role Profile
    {
        // Set the GAP Role Parameters
        let _ = GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, 1, &true as *const _ as _);
        let _ = GAPRole_SetParameter(
            GAPROLE_SCAN_RSP_DATA,
            SCAN_RSP_DATA.len() as _,
            SCAN_RSP_DATA.as_ptr() as _,
        );
        let _ = GAPRole_SetParameter(GAPROLE_ADVERT_DATA, ADVERT_DATA.len() as _, ADVERT_DATA.as_ptr() as _);
    }

    // Set the GAP Characteristics
    let _ = GGS_SetParameter(
        GGS_DEVICE_NAME_ATT,
        ATT_DEVICE_NAME.len() as _,
        ATT_DEVICE_NAME.as_ptr() as _,
    );

    // Setup the GAP Bond Manager
    {
        let passkey: u32 = 0; // passkey "000000"
        let pair_mode = GAPBOND_PAIRING_MODE_WAIT_FOR_REQ;
        let mitm = false;
        let io_cap = GAPBOND_IO_CAP_DISPLAY_ONLY;
        let bonding = true;
        let _ = GAPBondMgr_SetParameter(
            GAPBOND_PERI_DEFAULT_PASSCODE,
            size_of_val(&passkey) as _,
            &passkey as *const _ as _,
        );
        let _ = GAPBondMgr_SetParameter(GAPBOND_PERI_PAIRING_MODE, 1, &pair_mode as *const _ as _);
        let _ = GAPBondMgr_SetParameter(GAPBOND_PERI_MITM_PROTECTION, 1, &mitm as *const _ as _);
        let _ = GAPBondMgr_SetParameter(GAPBOND_PERI_IO_CAPABILITIES, 1, &io_cap as *const _ as _);
        let _ = GAPBondMgr_SetParameter(GAPBOND_PERI_BONDING_ENABLED, 1, &bonding as *const _ as _);
    }

    // Initialize GATT attributes
    {
        let _ = GGS_AddService(GATT_ALL_SERVICES).unwrap(); // GAP
        let _ = GATTServApp::add_service(GATT_ALL_SERVICES).unwrap(); // GATT attributes
    }

    // Add other service
}

async unsafe fn handle_tmos_event(event: &TmosEvent) {
    match event.message_id() {
        TmosEvent::GAP_MSG_EVENT => {
            // Peripheral_ProcessGAPMsg
            let msg = event.0 as *const gapRoleEvent_t;

            let opcode = unsafe { (*msg).gap.opcode };
            match opcode {
                GAP_SCAN_REQUEST_EVENT => {
                    log!(
                        "GAP scan request from {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ...",
                        (*msg).scanReqEvt.scannerAddr[0],
                        (*msg).scanReqEvt.scannerAddr[1],
                        (*msg).scanReqEvt.scannerAddr[2],
                        (*msg).scanReqEvt.scannerAddr[3],
                        (*msg).scanReqEvt.scannerAddr[4],
                        (*msg).scanReqEvt.scannerAddr[5],
                    );
                }
                GAP_PHY_UPDATE_EVENT => {
                    log!(
                        "GAP phy update Rx:{:x} Tx:{:x}",
                        (*msg).linkPhyUpdate.connRxPHYS,
                        (*msg).linkPhyUpdate.connTxPHYS,
                    );
                }
                GAP_LINK_PARAM_UPDATE_EVENT => {
                    log!(
                        "GAP link param update status: {:x} interval: {:x} latency: {:x} timeout: {:x}",
                        (*msg).linkUpdate.status,
                        (*msg).linkUpdate.connInterval,
                        (*msg).linkUpdate.connLatency,
                        (*msg).linkUpdate.connTimeout,
                    );
                }
                _ => {
                    log!("GAP MSG EVENT: {:p} {:x}", msg, opcode);
                }
            }
        }
        TmosEvent::GATT_MSG_EVENT => {
            let msg = event.0 as *const gattMsgEvent_t;
            let method = unsafe { (*msg).method };
            log!("GATT_MSG_EVENT: {:p} {:x}", msg, method);
        }
        _ => {
            log!("peripheral got event: {:?} id=0x{:02x}", event, event.message_id());
        }
    }
}


// App logic

pub enum AppEvent {
    Connected(u16),
    Disconnected(u16),
    BlinkySubscribed(u16),
    BlinkyUnsubscribed(u16),
}

static APP_CHANNEL: Channel<CriticalSectionRawMutex, AppEvent, 3> = Channel::new();

/// Default desired minimum connection interval (units of 1.25ms)
const DEFAULT_DESIRED_MIN_CONN_INTERVAL: u16 = 20;
/// Default desired maximum connection interval (units of 1.25ms)
const DEFAULT_DESIRED_MAX_CONN_INTERVAL: u16 = 160;
/// Default desired slave latency to use if parameter update request
const DEFAULT_DESIRED_SLAVE_LATENCY: u16 = 1;
/// Default supervision timeout value (units of 10ms)
const DEFAULT_DESIRED_CONN_TIMEOUT: u16 = 1000;

pub async fn peripheral(spawner: Spawner, task_id: u8, mut subscriber: ble::EventSubscriber) -> ! {
    // Profile State Change Callbacks
    unsafe extern "C" fn on_gap_state_change(new_state: gapRole_States_t, event: *mut gapRoleEvent_t) {
        log!("in on_gap_state_change: {}", new_state);
        let event = &*event;

        // state machine, requires last state
        static mut LAST_STATE: gapRole_States_t = GAPROLE_INIT;

        // time units 625us
        const DEFAULT_FAST_ADV_INTERVAL: u16 = 32;
        const DEFAULT_FAST_ADV_DURATION: u16 = 30000;

        const DEFAULT_SLOW_ADV_INTERVAL: u16 = 1600;
        const DEFAULT_SLOW_ADV_DURATION: u16 = 0; // continuous

        static mut CONN_HANDLE: u16 = INVALID_CONNHANDLE;

        match new_state {
            GAPROLE_CONNECTED => {
                // Peripheral_LinkEstablished
                if event.gap.opcode == GAP_LINK_ESTABLISHED_EVENT {
                    log!("connected.. !!");
                    CONN_HANDLE = event.linkCmpl.connectionHandle;

                    let _ = APP_CHANNEL.try_send(AppEvent::Connected(CONN_HANDLE));
                }
            }
            // if disconnected
            _ if LAST_STATE == GAPROLE_CONNECTED && new_state != GAPROLE_CONNECTED => {
                // link loss -- use fast advertising
                let _ = GAP_SetParamValue(TGAP_DISC_ADV_INT_MIN, DEFAULT_FAST_ADV_INTERVAL);
                let _ = GAP_SetParamValue(TGAP_DISC_ADV_INT_MAX, DEFAULT_FAST_ADV_INTERVAL);
                let _ = GAP_SetParamValue(TGAP_GEN_DISC_ADV_MIN, DEFAULT_FAST_ADV_DURATION);

                let _ = APP_CHANNEL.try_send(AppEvent::Disconnected(CONN_HANDLE));

                // Enable advertising
                let _ = GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, 1, &true as *const _ as _);
            }
            // if advertising stopped
            GAPROLE_WAITING if LAST_STATE == GAPROLE_ADVERTISING => {
                // if fast advertising switch to slow
                if GAP_GetParamValue(TGAP_DISC_ADV_INT_MIN) == DEFAULT_FAST_ADV_INTERVAL {
                    let _ = GAP_SetParamValue(TGAP_DISC_ADV_INT_MIN, DEFAULT_SLOW_ADV_INTERVAL);
                    let _ = GAP_SetParamValue(TGAP_DISC_ADV_INT_MAX, DEFAULT_SLOW_ADV_INTERVAL);
                    let _ = GAP_SetParamValue(TGAP_GEN_DISC_ADV_MIN, DEFAULT_SLOW_ADV_DURATION);
                    let _ = GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, 1, &true as *const _ as _);
                }
            }
            // if started
            GAPROLE_STARTED => {
                log!("initialized..");
                let mut system_id = [0u8; 8]; // DEVINFO_SYSTEM_ID_LEN
                GAPRole_GetParameter(GAPROLE_BD_ADDR, system_id.as_mut_ptr() as _).unwrap();

                // shift three bytes up
                system_id[7] = system_id[5];
                system_id[6] = system_id[4];
                system_id[5] = system_id[3];

                // set middle bytes to zero
                system_id[4] = 0;
                system_id[3] = 0;

                ptr::copy(system_id.as_ptr(), SYSTEM_ID.as_mut_ptr(), 8);
            }
            GAPROLE_ADVERTISING => {} // now advertising
            _ => {
                log!("!!! on_state_change unhandled state: {}", new_state);
            }
        }

        LAST_STATE = new_state;
    }

    // Deivce start
    unsafe {
        static BOND_CB: gapBondCBs_t = gapBondCBs_t {
            passcodeCB: None,
            pairStateCB: None,
            oobCB: None,
        };
        // peripheralStateNotificationCB
        static APP_CB: gapRolesCBs_t = gapRolesCBs_t {
            pfnStateChange: Some(on_gap_state_change),
            pfnRssiRead: None,
            pfnParamUpdate: None,
        };
        // Start the Device
        let r = GAPRole_PeripheralStartDevice(task_id, &BOND_CB, &APP_CB);
        log!("Start device {:?}", r);
    }

    loop {
        match select(subscriber.next_message_pure(), APP_CHANNEL.receive()).await {
            Either::First(event) => unsafe {
                handle_tmos_event(&event).await;
            }
            Either::Second(event) => match event {
                AppEvent::Connected(conn_handle) => unsafe {
                    // 1600 * 625 us
                    Timer::after(Duration::from_secs(1)).await; // FIXME: spawn handler

                    GAPRole_PeripheralConnParamUpdateReq(
                        conn_handle,
                        DEFAULT_DESIRED_MIN_CONN_INTERVAL,
                        DEFAULT_DESIRED_MAX_CONN_INTERVAL,
                        DEFAULT_DESIRED_SLAVE_LATENCY,
                        DEFAULT_DESIRED_CONN_TIMEOUT,
                        task_id,
                    )
                    .unwrap();
                },
                AppEvent::Disconnected(conn_handle) => unsafe {
                    GATTServApp::init_char_cfg(conn_handle, BLINKY_CLIENT_CHARCFG.as_mut_ptr());
                },
                AppEvent::BlinkySubscribed(conn_handle) =>  {
                    spawner.spawn(blinky_notification(conn_handle)).unwrap();
                },
                _ => {
                    // other event. just broadcast
                }
            },
        }
    }
}
