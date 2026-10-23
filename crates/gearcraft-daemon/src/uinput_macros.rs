//! Macro playback via Linux uinput (virtual keyboard).
//!
//! Requires write access to `/dev/uinput` (usually root or group `input`).
//! Falls back to logging when the device cannot be opened.
//!
//! Key names: single keys (`a`, `F8`, `enter`) or combos (`ctrl+shift+m`).

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::Duration;

use tracing::{info, warn};

use gearcraft_core::MacroDef;

// Minimal ioctl / uinput constants (linux/uinput.h, linux/input-event-codes.h)
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const SYN_REPORT: u16 = 0;
const UI_SET_EVBIT: u64 = 0x40045564;
const UI_SET_KEYBIT: u64 = 0x40045565;
const UI_DEV_SETUP: u64 = 0x405c5503;
const UI_DEV_CREATE: u64 = 0x5501;
const UI_DEV_DESTROY: u64 = 0x5502;
const BUS_USB: u16 = 0x03;

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

#[repr(C)]
struct InputEvent {
    time_sec: u64,
    time_usec: u64,
    type_: u16,
    code: u16,
    value: i32,
}

fn key_map() -> HashMap<&'static str, u16> {
    let mut m = HashMap::new();
    const LETTERS: &[(&str, u16)] = &[
        ("a", 30), ("b", 48), ("c", 46), ("d", 32), ("e", 18), ("f", 33),
        ("g", 34), ("h", 35), ("i", 23), ("j", 36), ("k", 37), ("l", 38),
        ("m", 50), ("n", 49), ("o", 24), ("p", 25), ("q", 16), ("r", 19),
        ("s", 31), ("t", 20), ("u", 22), ("v", 47), ("w", 17), ("x", 45),
        ("y", 21), ("z", 44),
    ];
    for (k, v) in LETTERS {
        m.insert(*k, *v);
    }
    m.insert("1", 2);
    m.insert("2", 3);
    m.insert("3", 4);
    m.insert("4", 5);
    m.insert("5", 6);
    m.insert("6", 7);
    m.insert("7", 8);
    m.insert("8", 9);
    m.insert("9", 10);
    m.insert("0", 11);
    m.insert("f1", 59);
    m.insert("f2", 60);
    m.insert("f3", 61);
    m.insert("f4", 62);
    m.insert("f5", 63);
    m.insert("f6", 64);
    m.insert("f7", 65);
    m.insert("f8", 66);
    m.insert("f9", 67);
    m.insert("f10", 68);
    m.insert("f11", 87);
    m.insert("f12", 88);
    m.insert("enter", 28);
    m.insert("return", 28);
    m.insert("esc", 1);
    m.insert("escape", 1);
    m.insert("space", 57);
    m.insert("tab", 15);
    m.insert("backspace", 14);
    m.insert("left", 105);
    m.insert("right", 106);
    m.insert("up", 103);
    m.insert("down", 108);
    m.insert("ctrl", 29);
    m.insert("control", 29);
    m.insert("leftctrl", 29);
    m.insert("rightctrl", 97);
    m.insert("shift", 42);
    m.insert("leftshift", 42);
    m.insert("rightshift", 54);
    m.insert("alt", 56);
    m.insert("leftalt", 56);
    m.insert("rightalt", 100);
    m.insert("meta", 125);
    m.insert("super", 125);
    m.insert("win", 125);
    m
}

fn parse_combo(spec: &str, map: &HashMap<&str, u16>) -> Result<Vec<u16>, String> {
    let mut codes = Vec::new();
    for part in spec.split('+') {
        let p = part.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        let code = map
            .get(p.as_str())
            .copied()
            .ok_or_else(|| format!("unknown key: {p}"))?;
        codes.push(code);
    }
    if codes.is_empty() {
        return Err("empty key sequence".into());
    }
    Ok(codes)
}

unsafe fn ioctl_int(fd: i32, req: u64, arg: u64) -> std::io::Result<i32> {
    let r = libc::ioctl(fd, req as _, arg);
    if r < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(r)
    }
}

fn emit(file: &mut std::fs::File, type_: u16, code: u16, value: i32) -> std::io::Result<()> {
    let ev = InputEvent {
        time_sec: 0,
        time_usec: 0,
        type_,
        code,
        value,
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &ev as *const _ as *const u8,
            std::mem::size_of::<InputEvent>(),
        )
    };
    file.write_all(bytes)?;
    Ok(())
}

/// Play a macro. Returns Ok(true) if uinput was used, Ok(false) if logged only.
pub fn play_macro(def: &MacroDef) -> Result<bool, String> {
    let map = key_map();
    let mut sequences: Vec<(Vec<u16>, u32)> = Vec::new();
    for action in &def.actions {
        let codes = parse_combo(&action.key, &map)?;
        sequences.push((codes, action.delay_ms));
    }

    match open_uinput_device(&map) {
        Ok(mut file) => {
            thread::sleep(Duration::from_millis(50));
            for (codes, delay) in &sequences {
                for &c in codes {
                    emit(&mut file, EV_KEY, c, 1).map_err(|e| e.to_string())?;
                }
                emit(&mut file, EV_SYN, SYN_REPORT, 0).map_err(|e| e.to_string())?;
                thread::sleep(Duration::from_millis(20));
                for &c in codes.iter().rev() {
                    emit(&mut file, EV_KEY, c, 0).map_err(|e| e.to_string())?;
                }
                emit(&mut file, EV_SYN, SYN_REPORT, 0).map_err(|e| e.to_string())?;
                if *delay > 0 {
                    thread::sleep(Duration::from_millis(*delay as u64));
                } else {
                    thread::sleep(Duration::from_millis(30));
                }
            }
            destroy_uinput(&mut file);
            info!("macro '{}' played via uinput ({} steps)", def.name, sequences.len());
            Ok(true)
        }
        Err(e) => {
            for (i, action) in def.actions.iter().enumerate() {
                info!(
                    "macro '{}' step {} (stub, no uinput: {}): key={}",
                    def.name, i, e, action.key
                );
            }
            Ok(false)
        }
    }
}

fn open_uinput_device(map: &HashMap<&str, u16>) -> Result<std::fs::File, String> {
    let mut file = OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .map_err(|e| format!("open /dev/uinput: {e}"))?;

    let fd = file.as_raw_fd();
    unsafe {
        ioctl_int(fd, UI_SET_EVBIT, EV_KEY as u64).map_err(|e| e.to_string())?;
        ioctl_int(fd, UI_SET_EVBIT, EV_SYN as u64).map_err(|e| e.to_string())?;
        for code in map.values() {
            let _ = ioctl_int(fd, UI_SET_KEYBIT, *code as u64);
        }
        for code in 1u64..256 {
            let _ = ioctl_int(fd, UI_SET_KEYBIT, code);
        }

        let mut setup: UinputSetup = std::mem::zeroed();
        setup.id.bustype = BUS_USB;
        setup.id.vendor = 0x0001;
        setup.id.product = 0x0001;
        setup.id.version = 1;
        let name = b"GearCraft Virtual Keyboard";
        setup.name[..name.len()].copy_from_slice(name);

        let r = libc::ioctl(fd, UI_DEV_SETUP as _, &setup);
        if r < 0 {
            return Err(format!("UI_DEV_SETUP: {}", std::io::Error::last_os_error()));
        }
        ioctl_int(fd, UI_DEV_CREATE, 0).map_err(|e| format!("UI_DEV_CREATE: {e}"))?;
    }
    Ok(file)
}

fn destroy_uinput(file: &mut std::fs::File) {
    let fd = file.as_raw_fd();
    unsafe {
        let _ = ioctl_int(fd, UI_DEV_DESTROY, 0);
    }
}
