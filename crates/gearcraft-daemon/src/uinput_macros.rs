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

use tracing::info;

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
    // All keys use &'static str literals — no temporary strings.
    const LETTERS: &[(&str, u16)] = &[
        ("a", 30),
        ("b", 48),
        ("c", 46),
        ("d", 32),
        ("e", 18),
        ("f", 33),
        ("g", 34),
        ("h", 35),
        ("i", 23),
        ("j", 36),
        ("k", 37),
        ("l", 38),
        ("m", 50),
        ("n", 49),
        ("o", 24),
        ("p", 25),
        ("q", 16),
        ("r", 19),
        ("s", 31),
        ("t", 20),
        ("u", 22),
        ("v", 47),
        ("w", 17),
        ("x", 45),
        ("y", 21),
        ("z", 44),
    ];
    const DIGITS: &[(&str, u16)] = &[
        ("1", 2),
        ("2", 3),
        ("3", 4),
        ("4", 5),
        ("5", 6),
        ("6", 7),
        ("7", 8),
        ("8", 9),
        ("9", 10),
        ("0", 11),
    ];
    const FUNCTION: &[(&str, u16)] = &[
        ("f1", 59),
        ("f2", 60),
        ("f3", 61),
        ("f4", 62),
        ("f5", 63),
        ("f6", 64),
        ("f7", 65),
        ("f8", 66),
        ("f9", 67),
        ("f10", 68),
        ("f11", 87),
        ("f12", 88),
    ];
    const SPECIAL: &[(&str, u16)] = &[
        ("enter", 28),
        ("return", 28),
        ("esc", 1),
        ("escape", 1),
        ("space", 57),
        ("tab", 15),
        ("backspace", 14),
        ("left", 105),
        ("right", 106),
        ("up", 103),
        ("down", 108),
        ("ctrl", 29),
        ("control", 29),
        ("leftctrl", 29),
        ("rightctrl", 97),
        ("shift", 42),
        ("leftshift", 42),
        ("rightshift", 54),
        ("alt", 56),
        ("leftalt", 56),
        ("rightalt", 100),
        ("meta", 125),
        ("super", 125),
        ("win", 125),
    ];

    let mut m = HashMap::new();
    for &(k, v) in LETTERS.iter().chain(DIGITS).chain(FUNCTION).chain(SPECIAL) {
        m.insert(k, v);
    }
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

    match open_uinput(&sequences) {
        Ok(mut file) => {
            for (codes, delay_ms) in &sequences {
                // Press all keys in combo
                for &code in codes {
                    emit(&mut file, EV_KEY, code, 1).map_err(|e| e.to_string())?;
                }
                emit(&mut file, EV_SYN, SYN_REPORT, 0).map_err(|e| e.to_string())?;
                thread::sleep(Duration::from_millis(20));
                // Release in reverse
                for &code in codes.iter().rev() {
                    emit(&mut file, EV_KEY, code, 0).map_err(|e| e.to_string())?;
                }
                emit(&mut file, EV_SYN, SYN_REPORT, 0).map_err(|e| e.to_string())?;
                if *delay_ms > 0 {
                    thread::sleep(Duration::from_millis(*delay_ms as u64));
                }
            }
            // Destroy device
            let fd = file.as_raw_fd();
            unsafe {
                let _ = ioctl_int(fd, UI_DEV_DESTROY, 0);
            }
            info!(
                "macro '{}' played via uinput ({} steps)",
                def.name,
                sequences.len()
            );
            Ok(true)
        }
        Err(e) => {
            info!(
                "uinput unavailable ({e}); logging macro '{}' ({} actions)",
                def.name,
                def.actions.len()
            );
            for a in &def.actions {
                info!("  → key={} delay_ms={}", a.key, a.delay_ms);
            }
            Ok(false)
        }
    }
}

fn open_uinput(sequences: &[(Vec<u16>, u32)]) -> std::io::Result<std::fs::File> {
    let file = OpenOptions::new().read(true).write(true).open("/dev/uinput")?;
    let fd = file.as_raw_fd();

    unsafe {
        ioctl_int(fd, UI_SET_EVBIT, EV_KEY as u64)?;
        ioctl_int(fd, UI_SET_EVBIT, EV_SYN as u64)?;
        // Enable every key we might emit
        for (codes, _) in sequences {
            for &code in codes {
                ioctl_int(fd, UI_SET_KEYBIT, code as u64)?;
            }
        }
        // Also enable a broad set so future macros work without re-open
        for code in 1u16..=255 {
            let _ = ioctl_int(fd, UI_SET_KEYBIT, code as u64);
        }
    }

    let mut setup = UinputSetup {
        id: InputId {
            bustype: BUS_USB,
            vendor: 0x0001,
            product: 0x0001,
            version: 1,
        },
        name: [0u8; 80],
        ff_effects_max: 0,
    };
    let name = b"GearCraft Virtual Keyboard";
    setup.name[..name.len()].copy_from_slice(name);

    unsafe {
        let r = libc::ioctl(fd, UI_DEV_SETUP as _, &setup as *const _);
        if r < 0 {
            return Err(std::io::Error::last_os_error());
        }
        ioctl_int(fd, UI_DEV_CREATE, 0)?;
    }
    // Give the kernel a moment to register the device
    thread::sleep(Duration::from_millis(50));
    Ok(file)
}
