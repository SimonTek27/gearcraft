# Gap Analysis: SteelSeries Sonar ↔ GearCraft

> Fonti (2026-10): Support article "Setting up Sonar for Streaming", "Getting to know your sonar settings",
> pagine prodotto steelseries.com/gg/sonar e /gg/sonar-for-streamers, review Stream Tech Reviews (BadIntent),
> capitoli tutorial ufficiali "How To: Setup Sonar (2025)" e "How to use Sonar: deep dive".
> Stato codebase: inventario verbatim da `crates/gearcraft-daemon` + `crates/gearcraft-ui` (vedi sezione 3).

---

## 1. Funzionalità ufficiali Sonar (elenco completo)

### 1.1 Setup wizard
- Selezione guidata di device per game / chat / media / aux / mic → Sonar crea le sezioni virtuali.
- "Game on!" finale → porta al tab Mixer.

### 1.2 Mixer tab
- Fader per **Master + Game + Chat + Media + Aux + Mic**.
- **Master = somma di tutti i canali**, con:
  - fader **Personal (🎧)** → headset/speaker dell'utente;
  - fader **Stream (📡)** → il pubblico;
  - **mute indipendente** di personal e stream;
  - icona **orecchio** → ascolti in cuffia esattamente ciò che sente il pubblico (monitor).
- **Streamer mode**: ogni canale mostra due fader (Personal/Stream). Il mix stream esce come
  **un solo device virtuale "Sonar Stream"** da scegliere in OBS.
- **Mute per canale** (temporaneo). **Solo** non esiste in Sonar (estensione nostra).
- Righe pulsanti per canale → tab dedicati; un canale disabilitato **non mostra il suo tab**
  finché non viene riattivato dal mixer.
- **Nascondi/mostra Media e Aux** (interchangeable) dal mixer.
- **ChatMix slider** in basso (bilanciamento globale game↔chat, non fader per canale).

### 1.3 Pannello ⚙ per canale
- Verifica formato audio del device del canale.
- **"Includi nel personal mix" e "Includi nel stream mix"** (toggle indipendenti
  → si può avere "sento io ma non il pubblico" e viceversa).
- **App-to-device routing manuale** (in Sonar apre il volume mixer di Windows; per noi
  assegnazione app→canale).
- **Shortcut (hotkey) per canale, separate per personal e stream.**

### 1.4 Tab per canale (FX)
| Tab | Contenuto |
|---|---|
| **Game** | EQ **parametrico** (curva, 5→10 punti, Q regolabile per banda, **low-cut**), slider basilari (bass/voice/treble), **Spatial audio** (slider performance↔immersion, distanza virtual speakers), **Volume Boost** (gain), **Smart Volume** (compressore leggero) |
| **Chat** | EQ + **noise reduction (background/impact) + noise gate + compressor** → serve anche per equalizzare la voce degli amici (in streamer-mode applica le stesse catene al canale nella stream mix) |
| **Media / Aux** | **identici e intercambiabili**: preset in alto a sinistra, testing station in alto a destra, grafico EQ, spatial audio |
| **Mic** | EQ 10 bande classico + **Noise Reduction (background + impact) + Noise Gate + Compressor** (manuali, mutuamente esclusivi con ClearCast) oppure **ClearCast AI NC** (unico slider che li sostituisce tutti), **mic test**: registra un campione e lo mette in loop mentre regoli live |

- **Presets**: lista in alto a sinistra in ogni tab ("many presets", incl. **Pro Presets per gioco**,
  es. preset CS2).

### 1.5 Impostazioni globali
- Lingua (multi-lingua), output device per personal mix, mic input + livello, integration DTS Engine per spatial audio.

### 1.6 Vincoli Sonar che il nostro app NON ha (vantaggi)
- Richiede account SteelSeries + login online.
- Windows-only.
- Gated su hardware semiproprietario per qualche funzione.

---

## 2. Stato attuale GearCraft (fatti verbatim dal codice)

### 2.1 Daemon
- **ChatMix** (`chatmix_pw.rs`): 4 null-sink via `pactl load-module module-null-sink`
  (`gearcraft_game/chat/media/aux`); volumi via `wpctl`/`pactl` (balance ≥0 → chat=100−b).
- **DSP** (`dsp_pw.rs`): backend `wpexec` (WirePlumber) → `module-filter-chain`, sink unico `gearcraft.dsp`.
  Nel graph (reale): EQ bq_peaking/bq_lowshelf, preamp bq_highshelf (clamp), bass boost 80 Hz,
  **spatial = approssimazione EQ** (3 filtri statici, nessun HRTF), lowpass, compressor LADSPA `sc4`
  (fallback soft builtin), crossfeed LADSPA `bs2b` (fallback soft). **Placeholder**: smart volume
  (solo +1.5 dB @1 kHz). **Non nel graph**: reverb, wideness, convolver (messaggio dice di usare
  EasyEffects/JDSP), limiter vero (solo clamp preamp).
- **App routing** (`app_routing.rs`): loop attivo ogni 2 s; classifica sink-input con regex substring
  builtin (chat 15, media 21, aux 4+obs, game 17) e `pactl move-sink-input`. **Nessuna config**, nessun
  riporto verso la UI.
- **Device manager** (`device_manager.rs`): enumerazione via `pactl list sinks`; per canale
  `module-loopback` virtual→physical (latency 20 ms); failover tick ogni 3 s; priority/excluded/policy;
  persist `device_manager.json`.
- **Mic FX** (`mic_fx.rs`): **NON compilato** (manca `mod mic_fx;` in main.rs; importa un tipo
  `MicFxState` che NON esiste in core) — e comunque solo mute/gain via pactl; highpass e gate erano segnaposto.
- **Preset rules + hotplug**: regole per-device; `hotplug.rs` auto-apply su USB;
  `game_presets.rs` (auto preset su avvio gioco) **NON compilato**.
- **Persistenza** (`config.rs`): `settings.json` contiene **solo** `{chatmix}`; `save_settings` esiste
  ma è `#[allow(dead_code)]` — mai chiamato. Preseti: `audio_presets.json` (8 built-in) + `preset_rules.json`.
- **Systemd**: unit system `packaging/gearcraft-daemon.service` (install.sh = system unit; AUR = user unit —
  incoerenza). **Nessun XDG autostart**, nessun codice per installarlo.

### 2.2 UI (Slint)
- Pagine: 0 Engine (lista device), 1 Sonar (mixer 6 canali fissa 832px, ChatMix, selector row
  Mixer/Game/Chat/Media/Aux/Mic → FX page), 2 Prism (RGB + DPI), 3 Camera (V4L2), 4 Cattura schermo,
  5 Preset/FX per canale (EQ interattivo 10 bande, Spatial/Boost, Mic FX UI).
- `ChannelRow` ha `volume`, `streamer-volume`, `pan`, `muted`, `solo`, `apps`, `preset-name/on`;
  streamer-mode default OFF; master dual-fader in streamer (logica ufficiale, fatti).
- **Callback UI → Rust senza cablaggio reale**: tutta la pagina FX (EQ band, spatial, boost,
  smart volume, reset), mic FX (gain, highpass, gate, lowpass, compressor), camera (~30),
  window chrome, screenshot/mouse DPI.
- `ChannelRow.apps` **mai popolato dal backend** (testo mostra `""` → fallback device).
- Master/media/aux/mic volume: **il fader non tocca il backend** (mode: solo game/chat via ChatMix state).
- Settings (autostart/lingua/tema): in-memory, `settings-autostart` non fa nulla.

---

## 3. Matrice di confronto e gap

Legenda: ✅ presente e funzionante · 🟡 presente UI ma non cablato/funzionale · ❌ assente · ⚪ n/a

| Area | Sonar | GearCraft | Gap |
|---|---|---|---|
| 6 canali virtuali + sink | Sonar Gaming/Chat/Media/Aux/Mic/Stream | 4 null-sink + mic non virtuale | 🟡 manca sink **mic**; manca sink **Stream** |
| Mixer fader per canale |izzo reale | UI fader tutti; backend solo game/chat (via chatmix volumes) | ❌ **P0** `SetChannelVolume` IPC + real wpctl per ogni canale (virtual sink o loopback gain) |
| Fader Personal/Stream (streamer) | due mix separati veri | `streamer-volume` UI-only; nessunmix stream nel backend | ❌ **P0** sink `gearcraft_stream` + loopback per canale con gain stream indipendente |
| Include in personal/stream | toggle ⚙ per canale | assente | ❌ **P1** campi `include-personal`/`include-stream` in `ChannelDevicePolicy`/ChannelRow + loopback on/off |
| Mute per canale | reale (entrambi mix) | `channel-mute` cablato a UI model solo | ❌ **P0** mute reale (pactl mute sul loopback/sink) |
| Master | somma + ear monitor + mute x2 | fader master decorativo; ear assente | ❌ **P1** master = volumi combinati + ear monitor (loopback stream-monitor→personal) |
| ChatMix | sì | ✅ reale (null-sink + wpctl) | — |
| Nascondi Media/Aux | sì | assente | 🟡 **P2** flag visibilità in DeviceManagerState |
| Tab canale disabilitato | nascosto | n/a (no hide) | segue P2 sopra |
| EQ | parametrico curva + Q + low-cut + basis sliders | UI 10 bande fisse interattive; backend EQ reale (bq_peaking) | 🟡 **P1** cablare on_toggle_eq/on_eq_band_gain → IPC + filter-chain rebuild; aggiungere low-cut (bq_highpass); Q regolabile |
| Spatial audio | HRTF-like reale | approssimazione EQ reale ma povera | 🟡 **P2** cablare callbacks; valutare convolver HRTF equip (EasyEffects style) |
| Volume Boost | reale | UI slider; backend preamp include boost già se entra in preset — non live | 🟡 **P1** cablare `set-volume-boost` → reload chain |
| Smart Volume | compressore leggero reale | placeholder (+1.5dB) | ❌ **P1** nodo compressore soft (LADSPA sc4 già per compressor) con strength 0→50% |
| Compressor/lowpass | mic: sì | **non cablato** nella UI FX; DSP ha compressore per output | 🟡 **P1** wiring + parametri |
| Noise reduction | background/impact | assente | ❌ **P2** — non fattibile AI-embedded; opzione: rimandare al punto 12 (rnnoise) |
| Noise gate | reale | assente (mic_fx placeholder) | ❌ **P1** mic filter-chain con gate (SPA builtin `gate` o LADSPA) |
| ClearCast AI NC | 1 slider AI | assente | ❌ **P2** su Linux: LV2 `rnnoise` (se installato) come "Noise Cancelling" slider; fallback in message |
| Mic test in loop | rec + loop playback | assente | ❌ **P2** `pw-cat --record`/`--playback` loop su source selezionata |
| Preset per canale | ogni tab lista preset | preset globali (8 built-in) applicati al DSP singolo | ❌ **P1 architetturale** — `effects` per canale: map `channel→AudioEffects` + N filter-chain (o una catena unica pre-canale inuna filter-chain per canale) |
| Pro Presets / auto preset per gioco | preset per gioco incluso CS2 + auto apply | `preset_rules` (device-based) + `game_presets.rs` **non compilato** | 🟡 **P1** compilare `game_presets.rs` + IPC toggle; regole già esistono |
| App routing | drag&drop manuale + routing di default | auto-routing activo (builtin rules); UI box vuoto | 🟡 **P1** IPC `GetChannelApps` → popolare `ChannelRow.apps`; **P2** override manuale per-app salvato |
| Testing station | sì (canale tab) | assente | ❌ **P2** tono/rumore rosa verso canale |
| Shortcut personal/stream | hotkey per canale | assente | ❌ **P2** (serve hooking globale; possibile `evdev`/uinput daemon) |
| Wizard iniziale | device pick per canale | device manager esiste ma non wizard | 🟡 **P2** onboarding che chiama DeviceManagerState default |
| Lingua | multi-lingua | selector presente ma nessuna traduzione attiva | ❌ **P2** stringhe i18n (Slint(translatable)) + persist |
| Tema | dark per default/temi | selector in-memory; runtime switch non applicabile (Slint 1.6) | 🟡 persist e riavvio applicato |
| Statistiche device/audio format | mostra formato canale | no | ⚪ **P2** info da `pactl list sinks/source` |
| Autostart | n/a (Windows startup) | toggle in-memory senza effetto | ❌ **P1** salvataggio settings via `save_settings` + install `.desktop` in `~/.config/autostart` + resolve uniti systemd (coerenza system/user) |

### Cose da NON replicare
- **Account/login online**: gearcraft non ne ha — vantaggio da mantenere e documentare.
- **DTS Engine integration**: n/a su PipeWire (sostituibile da spatial nuestro).
- **ClearCast AI**: algoritmo proprietario — sostituti open-source per Linux (rnnoise/LV2, builtin SPA gate).

---

## 4. Roadmap di chiusura gap (priorità)

### P0 — il mixer "mente" al utente (fondamento Sonar)
1. **Volumi reali per canale**: IPC `SetChannelVolume(channel, personal, stream, muted)`
   → `wpctl set-volume <virtual_sink>` per personal; per stream: усare il loopback volume
   (`wpctl set-volume` sul loopback node) con moduli memorizzati in state.
2. **Stream mix reale**: `setup_virtual_sinks()` crea anche `gearcraft_stream`;
   quando streamer-mode ON → per canale loopback `monitor → gearcraft_stream`
   con gain `streamer-volume`; OFF → rimuovi stream loopback.
3. **Mute reale per canale** (`pactl set-sink-mute`/`set-source-mute` sul loopback o wpctl node id).

### P1 — portata Sonar full
4. **Persistenza settings** (`save_settings` esteso: autostart, language, theme, rgb-enabled,
   streamer-mode, include-flags, hide-media/aux) + **XDG autostart installer** nel daemon
   (scrive `~/.config/autostart/gearcraft.desktop`) + risoluzione incoerenza systemd system/user
   tra `install.sh` e PKGBUILD.
5. **_include-personal/include-stream per canale** (+ 🎧 che scompare se canale escluso).
6. **Cablaggio pagina FX**: EQ band/lowcut/Q, spatial (debounce reload chain), volume boost,
   smart volume vero (compressore soft), compressor/lowpass UI wiring; reload filter-chain debounce!
   (già esiste kill/spawn wpexec — aggiungere debounce 300ms per evitare churn).
7. **Preset per canale**: ristrutturare `AudioEffects` bucket per canale con default ereditate;
   filter-chain compile per canale; preset manager salva layout nuovo (migra i vecchi).
8. **App list per canale in UI**: IPC `GetChannelApps` (sink-inputs → app name + icon letter) +
   refresh su evento (app_routing pass diff) — popola `ChannelRow.apps` in tempo reale.
9. **EAR monitor su master** (loopback stream.monitor → device personal, volume 100%, toggle).
10. **Auto-preset per gioco**: compilare `game_presets.rs` + toggle settings.

### P2 — nicchie e polish
11. Nascondi/mostra Media-Aux (flag `visible` in ChannelDevicePolicy + non creare sink/loopback).
12. Noise cancelling (rnnoise LV2 optional; slider unito) + gate reale mic + mic test loop.
13. Testing station (tono/campioni per canale).
14. Shortcut globali per canale (daemon uinput/evdev listener; config JSON).
15. Onboarding wizard (choose default device per canale).
16. i18n reale (<Sistema> = env LANG mapping; tradurre stringhe chiave) + tema persist.
17. Audio device format info nel pannello ⚙ canale.

---

## 5. Implementation notes (Linux/PipeWire)
- Loopback volume/mute: `pactl list modules` → module index of `module-loopback`; node id via
  `wpctl status` parsing (esattamente la tecnica già usata in `chatmix_pw::detect_sinks`).
- Il numero di filter-chain (per canale) moltiplica wpexec child: preferire **una filter-chain per
  canale attivo** con FX ≠ flat, passthrough altrimenti. wpexec spawn costa ~150ms — debounce.
- Streamer-mode OFF: rimuovere loopback stream e distruggere `gearcraft_stream` per
  non inquinare ol'app-switcher audio del desktop.
- `save_settings(settings: AppSettings)` in config.rs già accetta JSON — estendere la struct.
