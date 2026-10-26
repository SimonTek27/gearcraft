# GearCraft — UI Guidelines (Slint 1.6)

> Documento di riferimento per mantenere stabile l'interfaccia Slint.  
> **La UI è considerata definitiva** (concept bloccato). Qualsiasi modifica futura deve rispettare le regole qui sotto per non rompere layout, build o comportamento.

---

## 1. Stack e versioni

| Componente | Versione |
|------------|----------|
| Slint | `=1.6.0` (pinned in `crates/gearcraft-ui/Cargo.toml`) |
| Stile widget | `fluent-dark` (impostato in `crates/gearcraft-ui/build.rs`) |
| Rust edition | 2021 |
| Accent color | `#00E7A0` (verde GearCraft) |
| Background | `#0F1013` / pannelli `#1A1B1E` / bordi `#2A2B2F` |

> ⚠️ **Non aggiornare Slint oltre 1.6.0** senza testare: la sintassi dei componenti, le proprietà `vertical-alignment`, i layout e il codice generato cambiano. I 23 warning `std::usize::MAX` nel file generato (`target/.../out/app.rs`) sono attesi e **non fixabili** nel codice progetto.

---

## 2. File chiave

| File | Ruolo |
|------|-------|
| `crates/gearcraft-ui/ui/app.slint` | Definizione UI (unico file Slint) |
| `crates/gearcraft-ui/src/main.rs` | Callback Rust ↔ Slint, refresh modelli |
| `crates/gearcraft-ui/src/ipc.rs` | Client IPC verso il daemon |
| `crates/gearcraft-ui/build.rs` | Config compilazione Slint (`fluent-dark`) |

---

## 3. Architettura delle pagine

`page` è un `int` (0–5). Le pagine sono condizionali `if root.page == N`.

| page | Nome | Contenuto |
|------|------|-----------|
| 0 | Engine | Lista dispositivi + refresh + pulsanti rapidi |
| 1 | Sonar | Mixer canali + ChatMix |
| 2 | Prism | RGB per device selezionato (effetti, zone, colore, DPI) |
| 3 | Camera | Preview V4L2, capture, effetti |
| 4 | Cattura | Screenshot Spectacle |
| 5 | Preset/FX | Equalizer, Spatial/Boost, Mic FX |

Navigazione: click su device (Engine) → `root.page = 2`. Click preset (Sonar) → `root.page = 5`. Pulsanti "←" riportano alla pagina precedente.

---

## 4. Regole di layout (NON violare)

### 4.1 Mixer (Sonar)
- Il mixer è un `HorizontalBox` con **`alignment: start`** e **senza `vertical-stretch`**.
- I canali hanno **`width: 132px` fisso** (mai `min-width` + stretch).
- L'altezza del fader è **fissa a 120px**.
- **Non aggiungere `vertical-stretch: 1`** al mixer né ai canali: romperebbe le proporzioni al resize.
- L'`HorizontalBox` del mixer deve restare **allineato a sinistra** (`alignment: start`).

### 4.2 Sidebar
- Larghezza fissa **200px**.
- **Nessun logo/titolo/status** nella sidebar: il logo "GearCraft" e lo status vivono solo nella **header bar** (title bar).
- La sidebar contiene solo i `NavItem` (spacing 8px, padding 16px).
- Gli `NavItem` usano uno slot icona fisso 18×18 px (vedi §5).

### 4.3 Finestra
- **Non riaggiungere `no-frame: true`**: su Linux/X11 rende la finestra non ridimensionabile.
- `min-width: 900px`, `min-height: 600px`.
- La header bar contiene i controlli finestra (minimize/maximize/close) e il logo.

### 4.4 ChatMix
- Sotto il mixer in Sonar deve esserci **solo** la barra ChatMix + uno spacer di 24px.
- Non reinserire Audio presets o Device Manager nella pagina Sonar (vanno su pagine dedicate).

### 4.5 Empty space
- Lo spacer sotto ChatMix è `Rectangle { height: 24px; background: transparent; }`.

---

## 5. Componenti riutilizzabili

### 5.1 `NavItem`
```slint
component NavItem inherits Rectangle {
    in property <string> label;
    in property <bool> active;
    callback clicked();
    // icona passata come @children dentro uno slot 18x18
}
```
- L'icona va passata come figlio dentro lo slot fisso (non modificare il layout interno).
- `active` controlla il colore accent.

### 5.2 `ConsoleButton`
Sostituisce `Button` di std-widgets. API:
```slint
in property <string> text: "";
in property <bool> primary: false;
in property <bool> enabled: true;
callback clicked();
```
- **Usare sempre `ConsoleButton`**, mai `Button` di std-widgets (rimosso dall'import).
- Per pulsanti inline su una riga: `ConsoleButton { text: "..."; clicked => { ... } }`.

### 5.3 `ToggleSwitch`
```slint
component ToggleSwitch inherits Rectangle {
    in property <bool> checked: false;
    in property <string> label: "";
    callback toggled();
}
```
- Stile Android: track arrotondato + thumb scorrevole, verde quando attivo.
- Usare per toggle booleani (streamer mode, ecc.), **non** per pulsanti d'azione.

### 5.4 `ChannelRow` (struct)
Campi obbligatori (non rimuovere né rinominare):
```
id, label, color, volume, muted, solo, device,
device-volume, preset-name, preset-on, apps, pan
```
- `volume` = volume canale (fader verticale)
- `device-volume` = volume device (anello di progresso)
- `pan` = bilanciamento L/R (-100..100, default 0)
- `preset-on` = toggle preset sul canale

---

## 6. Callbacks e binding Rust ↔ Slint

Tutte le callback sono dichiarate in `app.slint` e collegate in `main.rs`.  
**Regola**: ogni nuova callback deve essere:
1. Dichiarata in `app.slint` (`callback nome(...)`).
2. Cablata in `main.rs` con `ui.on_nome(move |...| { ... })`.
3. Se manipola il modello `channels`, usare `update_channel(ui, id, |ch| { ... })`.

Callback critiche (non rimuovere):
- `refresh-devices`, `select-device`, `apply-color`
- `channel-volume`, `channel-mute`, `channel-solo`, `channel-pan`
- `toggle-streamer`, `toggle-sonar`, `set-chatmix`
- `toggle-preset-rule`, `toggle-channel-preset`
- `link-all-outputs`, `refresh-device-manager`, `apply-device-manager`
- `eq-band-gain`, `toggle-eq`, `apply-eq`, `reset-eq`
- `set-spatial-immersion`, `set-spatial-distance`, `set-volume-boost`, `toggle-spatial`, `toggle-smart-volume`, `apply-spatial`
- `set-mic-gain`, `toggle-mic-mute`, `toggle-mic-highpass`, `toggle-mic-gate`, `toggle-mic-lowpass`, `toggle-mic-compressor`, `apply-mic-fx`, `refresh-mic-fx`
- `toggle-camera-preview`, `apply-camera-settings`, `add-camera-effect`, `remove-camera-effect`, `toggle-camera-effect`, `set-camera-effect-intensity`
- `win-minimize`, `win-maximize`, `win-close`, `open-settings`, `open-help`

---

## 7. Pitfalls Slint 1.6 (errori comuni)

| Errore | Causa | Fix |
|--------|-------|-----|
| `padding: 10px 8px` non valido | Slint non supporta shorthand CSS | Usare `padding-top/bottom/left/right` |
| `color: string` vs `brush` | `ChannelRow.color` è `brush` | Passare `#hex` (brush), non stringa |
| `#hex` tra virgolette | Deve essere letterale | `color: #00e7a0` senza virgolette |
| `FnOnce` vs `FnMut` | Closure mutabili | Usare `FnMut` in `update_channel` |
| `ModelRc` senza `.row_count()` | Serve trait `Model` | `use slint::Model;` |
| `vertical-alignment` su `Rectangle`/`HorizontalBox` | Proprietà non esiste su quei tipi | Usarla solo su `Text` |
| `Button` da std-widgets | Rimosso dall'import | Usare `ConsoleButton` |
| Doppio `let ui_weak` | Variabile inutilizzata | Rimuovere il duplicato |
| Anello di progresso | Usare il pattern `Path` con `MoveTo`/`ArcTo` (vedi device-volume) |

---

## 8. Comandi di build e test

```bash
# Build completa
cargo build --workspace

# Solo UI
cargo build -p gearcraft-ui

# Solo daemon
cargo build -p gearcraft-daemon

# Test
cargo test --workspace

# Daemon modalità demo (senza hidapi)
cargo build -p gearcraft-daemon --no-default-features

# Avvio
cargo run -p gearcraft-daemon &
cargo run -p gearcraft-ui
```

Verifica prima di ogni commit:
```bash
cargo build --workspace && cargo test --workspace
```

---

## 9. Checklist pre-commit

- [ ] `cargo build --workspace` senza errori
- [ ] `cargo test --workspace` verde
- [ ] Nessun `Button` di std-widgets reintrodotta (usare `ConsoleButton`)
- [ ] Nessun `no-frame: true` reintrodotta
- [ ] Mixer con `alignment: start`, canali con `width` fisso, nessun `vertical-stretch` sul mixer
- [ ] Sidebar senza logo/titolo, solo NavItem
- [ ] Sotto il mixer in Sonar solo ChatMix + spacer
- [ ] `ChannelRow` con tutti i campi (inclusi `pan`, `device-volume`, `preset-on`)
- [ ] Nuove callback cablate in `main.rs`
- [ ] Push su `origin/main`

---

## 10. Bug noti / da risolvere (concept definitivo)

> La UI è considerata definitiva a livello di concept. I seguenti punti sono bug da sistemare in iterazioni future, **senza cambiare il layout**:

1. **Preset on/off per canale** — il toggle `preset-on` aggiorna solo lo stato locale; va collegato al daemon (set/delete preset rule per route).
2. **App routing** — il riquadro "App" sotto ogni canale mostra il device se `apps == ""`; va popolato con le app effettivamente instradate (IPC `list routed apps`).
3. **Pan (L/R)** — lo slider aggiorna il modello ma non è ancora applicato all'audio (manca IPC `set-channel-pan`).
4. **Device Manager** — rimosso dalla pagina Sonar; valutare se reintrodurlo in una pagina dedicata o in Settings.
5. **Audio presets** — rimossi da Sonar; la gestione preset è su pagina 5 (Preset/FX) e via lista preset nel daemon.
6. **Slider `enabled` su ConsoleButton** — il campo `enabled` esiste ma non è sempre propagato (alcuni pulsanti non lo usano).
7. **Slint 1.6 warnings** — 19× `std::usize::MAX` nel codice generato; risolvibili solo aggiornando Slint.
8. **Settings panel** — contiene toggles per "Esegui all'avvio del sistema", lingua (`<sistema>` default) e tema (Dark / Light / Sistema / Personalizzato). Persistenza attualmente in-memory: da collegare a IPC `save_settings` (esiste in `config.rs`). Cambio tema runtime non applicabile con Slint 1.6 (stile fissato a compile-time in `build.rs`).

---

## 11. Contatti / manutenzione

- Repo: `https://github.com/SimonTek27/gearcraft`
- Branch principale: `main`
- Socket IPC: `/run/gearcraft/gearcraft.sock` (override via `GEARCRAFT_SOCKET`)
