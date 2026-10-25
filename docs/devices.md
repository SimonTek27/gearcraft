# Supported devices

GearCraft is **brand-agnostic**. Support is added through `PeripheralAddon` plugins.  
This table is the living matrix of targets and status. Categories follow typical PC peripheral / system roles.

**Status legend**

| Status | Meaning |
|--------|---------|
| ✅ | Implemented (live HID or solid protocol path) |
| 🟡 | Partial / demo / protocol sketch |
| ⬜ | Planned |
| — | Out of scope for this addon family |

---

## Input – mice

| Brand | Model | VID | PID | RGB | DPI / polling | Macros | Status | Addon |
|-------|-------|-----|-----|-----|---------------|--------|--------|-------|
| SteelSeries | Rival 3 | `0x1038` | `0x1824` | ✅ zones | 🟡 | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Rival 3 (fw 0.37) | `0x1038` | `0x184C` | 🟡 | 🟡 | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Rival 3 Wireless | `0x1038` | `0x1830` | 🟡 | 🟡 | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Rival 3 Gen 2 | `0x1038` | `0x1870` | 🟡 | 🟡 | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Rival 3 Wireless Gen 2 | `0x1038` | `0x1872` | 🟡 | 🟡 | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Rival 5 | `0x1038` | `0x183C` | ✅ zones | 🟡 | ⬜ | 🟡 | `steelseries` |
| Razer | (DeathAdder / Viper family) | `0x1532` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Logitech | (G Pro / G502 family) | `0x046D` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Corsair | (M65 / Sabre family) | `0x1B1C` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |

---

## Input – keyboards

| Brand | Model | VID | PID | RGB | Profiles | Macros | Status | Addon |
|-------|-------|-----|-----|-----|----------|--------|--------|-------|
| SteelSeries | Apex 5 | `0x1038` | `0x1612` | 🟡 uniform | ⬜ | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Apex 3 | `0x1038` | `0x161A` | 🟡 | ⬜ | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Apex 7 | `0x1038` | `0x161C` | 🟡 | ⬜ | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Apex 3 TKL | `0x1038` | `0x1622` | 🟡 | ⬜ | ⬜ | 🟡 | `steelseries` |
| Razer | (BlackWidow / Huntsman) | `0x1532` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Logitech | (G915 / G Pro) | `0x046D` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Corsair | (K70 / K100) | `0x1B1C` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |

---

## Audio – headsets & sound

| Brand | Model | VID | PID | Battery | ChatMix / software mix | EQ | Status | Addon |
|-------|-------|-----|-----|---------|--------------------------|-----|--------|-------|
| SteelSeries | Arctis 7 | `0x1038` | `0x12AD` | 🟡 | ✅ (PipeWire) | ⬜ | 🟡 | `steelseries` + ChatMix |
| SteelSeries | Arctis family | `0x1038` | `0x12BE`, `0x1260` | 🟡 | ✅ | ⬜ | 🟡 | `steelseries` |
| SteelSeries | Nova 7 | `0x1038` | `0x2202` | 🟡 | ✅ | ⬜ | 🟡 | `steelseries` |
| — | Any (software ChatMix) | — | — | — | ✅ Game/Chat sinks | — | ✅ | `chatmix_pw` |
| Razer | (BlackShark / Kraken) | `0x1532` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Logitech | (G Pro X / Astro) | `0x046D` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |
| Corsair | (HS / Virtuoso) | `0x1B1C` | — | ⬜ | ⬜ | ⬜ | ⬜ | planned |

---

## Motherboard / onboard RGB

| Brand | Scope | RGB zones | Status | Addon |
|-------|--------|-----------|--------|-------|
| ASUS | Aura / Armoury Crate paths | ⬜ | ⬜ | planned |
| MSI | Mystic Light | ⬜ | ⬜ | planned |
| Gigabyte | RGB Fusion | ⬜ | ⬜ | planned |
| ASRock | Polychrome | ⬜ | ⬜ | planned |
| OpenRGB-compatible | Shared protocol bridge | ⬜ | ⬜ | under consideration |

Motherboard support is **not** required for core peripheral control. It is listed for a future unified lighting hub.

---

## Videocard / GPU lighting

| Brand | Scope | Status | Addon |
|-------|--------|--------|-------|
| NVIDIA | GPU LEDs (limited public API) | ⬜ | planned / research |
| AMD | GPU LEDs | ⬜ | planned / research |
| Aorus / MSI Gaming / ASUS TUF boards | Board-level LEDs | ⬜ | often via motherboard addon |

GPU lighting is secondary; prefer OpenRGB or vendor tools where GearCraft has no stable path yet.

---

## Other / system

| Category | Examples | Status |
|----------|----------|--------|
| Controllers | Gamepads with RGB | ⬜ |
| Case fans / AIO | Addressable ARGB (via MB or USB hub) | ⬜ (often motherboard) |
| Webcams / stream decks | Non-RGB control | ⬜ |

---

## Notes for contributors

1. **Category first** – open a PR under the right section (mice, keyboards, audio, …).
2. **Addon isolation** – one crate/module per brand or protocol family implementing `PeripheralAddon`.
3. **udev** – add VID/PID to `assets/udev/99-gearcraft.rules` when enabling live HID.
4. **Protocol sources** – document independent research (e.g. open reverse-engineering projects under compatible licenses). Do not copy proprietary vendor apps.
5. **Software features** (ChatMix, macros via uinput) apply across brands and do not require a specific vendor device.

For HID protocol details of the current SteelSeries family implementation, see the `steelseries` addon sources under `crates/gearcraft-daemon/src/addons/steelseries/`.
