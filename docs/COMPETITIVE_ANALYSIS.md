# GearCraft vs concorrenza

Analisi funzionale (Linux desktop gaming + audio). Aggiornata 2026-10-26.

## Posizionamento

| Prodotto | Piattaforma | Focus |
|----------|-------------|-------|
| **GearCraft** | Linux (Rust) | Hub multi-brand: RGB + Sonar-like audio + utilità |
| **SteelSeries GG** | Win/mac | Engine + Sonar + Moments (chiuso) |
| **JamesDSP / JDSP4Linux** | Linux | DSP audio avanzato |
| **EasyEffects** | Linux | DSP PipeWire (GTK) |
| **OpenRGB** | Multi | RGB multi-vendor |
| **OpenRazer** | Linux | Driver/daemon Razer |
| **Piper / libratbag** | Linux | Mouse (DPI, button) |
| **Spectacle** | KDE/Linux | Screenshot |

GearCraft mira a **unificare** pezzi di GG + JDSP + OpenRGB su Linux, non a sostituire ognuno al 100%.

---

## Confronto per area

### 1. RGB / Engine (vs SteelSeries Engine + OpenRGB)

| Feature | GearCraft | SS GG Engine | OpenRGB |
|---------|-----------|--------------|---------|
| Lista device | Sì (HID + demo) | Sì | Sì |
| Colore per zona | Sì | Sì | Sì |
| Effetti (rainbow/breathe…) | Parziale | Sì | Sì |
| Per-key keyboard | No | Sì (Prism) | Sì (device-dependent) |
| PrismSync multi-device | No | Sì | Sì (SDK) |
| Backend ufficiale Linux | OpenRazer + OpenRGB CLI | — | Nativo |
| GameSense / reactive game | No | Sì | Plugin limitati |

**Manca:** editor per-key, sync multi-device nativo, effetti avanzati Apex, integrazioni gioco.

### 2. Audio / Sonar (vs SteelSeries Sonar + JamesDSP)

| Feature | GearCraft | Sonar | JamesDSP |
|---------|-----------|-------|----------|
| Mixer multi-canale | Sì (game/chat/media/aux) | Sì | No (system-wide) |
| ChatMix | Sì (null sinks) | Sì | No |
| App routing automatico | Sì (pactl class) | Sì | Allow/block list |
| Device Manager per canale | Sì (base) | Sì | Profili device |
| EQ 10 bande | Sì | Sì | Sì (più flessibile) |
| Spatial / immersion | Approssimato | Sì (360) | Surround/wideness |
| Mic FX (HPF/gate/comp) | Sì (base) | Sì (ClearCast AI) | Limitato |
| Preset + auto game | Sì | Sì | Preset + rules |
| Import preset JDSP | Sì (.conf) | No | Nativo |
| Convolver / AutoEQ / DDC | No | No | Sì |
| Streamer dual path | UI parziale | Sì | No |

**Manca:** AI noise reduction mic, spatial “vero”, convolver, AutoEQ, parity effetti JDSP, metering professionale.

### 3. DSP engine (vs JamesDSP / EasyEffects)

| Feature | GearCraft | JDSP | EasyEffects |
|---------|-----------|------|-------------|
| PipeWire integration | WirePlumber filter-chain | Sì | Sì |
| EQ | Builtin peaking | FIR/IIR/GraphicEQ | LADSPA/LV2 |
| Crossfeed BS2B | Sì se plugin | Sì | Sì |
| Compressor | sc4 / soft | Multiband | Sì |
| Limiter | Sì | Sì | Sì |
| Reverb / convolver | Flag / no | Sì | Sì |
| Live scripting | No | EEL2 | No |

**Manca:** profondità effetti e qualità DSP da “studio”.

### 4. Utility (vs Spectacle / GG Moments)

| Feature | GearCraft | Spectacle | GG Moments |
|---------|-----------|-----------|------------|
| Screenshot full/window/region | Sì | Sì | — |
| Annotazioni | No | Sì | — |
| Delay / clipboard | Sì | Sì | — |
| Clip recording gioco | No | Parziale | Sì |
| Camera page | Sì (base) | — | — |

**Manca:** annotazioni, history gallery, instant replay.

### 5. Mouse / tastiera oltre RGB

| Feature | GearCraft | Piper | SS GG |
|---------|-----------|-------|-------|
| DPI stages | No | Sì | Sì |
| Button remap | No | Sì | Sì |
| Macro engine | Parziale (store + uinput) | Limitato | Sì |
| Actuation (OmniPoint) | No | No | Sì |

**Manca:** DPI/polling UI, remap, support tastiera avanzato.

---

## Bug / debiti tecnici trovati e corretti in questo passaggio

1. **ChatMix volumes** — `unwrap()` su sink id → ora `ok_or` con errore esplicito.
2. **Demo devices** — non mascherano più uno scan vuoto; solo con `GEARCRAFT_DEMO=1`.
3. **Doc IPC** — commento TakeScreenshot/ImportJamesDsp ripristinato.

## Altri rischi noti (non ancora fixati)

- Prism UI: effetti Wave/Reactive su Razer mappati a Spectrum (limite protocollo attuale).
- Mic high-pass/gate reali dipendono da wpexec + swh-plugins.
- Engine: click device apre Prism, ma non tutti i PID hanno zone RGB.
- Camera/macros: superfici ampie, qualità hardware-dependent.
- Nessun test e2e automatizzato su HID reale (richiede macchina utente).
- Packaging AUR/Flatpak incompleto.

---

## Priorità suggerite (gap → impatto)

1. **DPI + button remap** (Piper/libratbag) — completa il lato “Engine mouse”.
2. **PrismSync** leggero via OpenRGB SDK (un effetto su N device).
3. **Convolver opzionale** (carica IR WAV) per avvicinarsi a JDSP.
4. **Mic AI/rnnoise** opzionale (PipeWire `libpipewire-module-filter-chain` + rnnoise).
5. **Annotazioni screenshot** o integrazione diretta `spectacle`.
6. **AUR package** + udev rules documentate.

## Conclusione

GearCraft copre già il **nucleo Linux** che GG non offre (Sonar-like + RGB + JDSP import + capture) ma resta **dietro** su:

- profondità DSP (JDSP/EasyEffects)
- RGB pro (per-key, sync)
- configurazione mouse (DPI/bottoni)
- polish UI e packaging

Il valore distintivo resta: **un solo daemon/UI nativo Linux multi-brand**, non cinque app separate.
