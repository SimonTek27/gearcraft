# Hardware testing

Live USB scanning is **enabled by default** (pure-Rust `linux-static-hidraw`
backend — no system libraries). You still need udev access and a real device.

## Quick checklist

```bash
# 1. udev rules (once)
sudo cp assets/udev/99-gearcraft.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
sudo usermod -aG plugdev "$USER"   # re-login after this

# 2. Confirm the device is visible to the kernel
lsusb -d 1038:                    # SteelSeries VID
ls -l /dev/hidraw*                # should be group plugdev or mode allows you

# 3. Scan only (no IPC server)
GEARCRAFT_SCAN_ONLY=1 cargo run -p gearcraft-daemon

# 4. Run daemon live
cargo run -p gearcraft-daemon

# 5. Optional: install permanently
sudo -E ./packaging/install.sh
sudo systemctl enable --now gearcraft-daemon
```

Demo-only build (no HID): `cargo build -p gearcraft-daemon --no-default-features`.

Or use the helper script:

```bash
./packaging/hw-test.sh            # checks + scan
./packaging/hw-test.sh --rgb      # + colour smoke test via IPC (daemon must be running)
```

## Environment

| Variable | Effect |
|----------|--------|
| `GEARCRAFT_SCAN_ONLY=1` | Enumerate HID devices, print report, exit |
| `GEARCRAFT_DRY=1` | Skip real USB (demo devices only) |
| `RUST_LOG=debug` | Verbose HID write/read logs |

## Per-device smoke tests

### Mouse (Rival 3 / Rival 5 family)

1. `GEARCRAFT_SCAN_ONLY=1 …` shows the mouse with a path under `/dev/hidraw*`.
2. Start the daemon.
3. From the UI **Engine**, set logo / zones to accent `#00E7A0`.
4. Or via IPC (socket `$XDG_RUNTIME_DIR/gearcraft.sock`):

```json
{
  "id": "00000000-0000-0000-0000-000000000001",
  "cmd": {
    "name": "set_color",
    "params": {
      "device_id": "ss-1038-1824-3",
      "zone": "logo",
      "color": { "r": 0, "g": 231, "b": 160 }
    }
  }
}
```

Expected: logo LEDs change; daemon log shows `HID write …`.

### Keyboard (Apex 3/5/7)

Same flow with zone `"all"` / uniform colour report. Per-key RGB is not implemented yet.

### Headset (Arctis / Nova 7)

1. Scan must list the headset.
2. On initialize the daemon issues battery query `06 18` and tries to read a response.
3. UI **Sonar** should show battery if the parse succeeds.
4. ChatMix is software (PipeWire) — independent of HID; see [CHATMIX.md](CHATMIX.md).

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Scan finds 0 devices | Built with `--no-default-features` | Rebuild with default features |
| Scan finds 0, `lsusb` shows device | Permission on hidraw | udev rules + `plugdev` + re-login |
| `Permission denied` on write | Same | `ls -l /dev/hidrawN`; fix group/mode |
| Colour does nothing | Wrong interface selected | Check the log line `HID candidate … score=…`; GearCraft picks the lowest interface with a vendor usage page (`0xFFC0`) |
| Battery always demo | Read timeout / wrong PID | Capture USB with Wireshark; open an issue |
| Demo devices still appear | Empty scan → fallback | Expected when no live match |

## Capture for new PIDs

```bash
# Install wireshark / usbmon
sudo modprobe usbmon
# Record while using the official Windows/mac app in a VM, or rivalcfg on Linux
```

Add the PID to `usb::KNOWN_STEELSERIES` and the right family helper in `addons/steelseries/`, then extend [devices.md](devices.md).
