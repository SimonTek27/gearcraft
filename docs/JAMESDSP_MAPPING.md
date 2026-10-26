# Mappatura dettagliata parametri JamesDSP → GearCraft

Riferimento: preset `.conf` di [JDSP4Linux](https://github.com/Audio4Linux/JDSP4Linux)
(`~/.config/jamesdsp/presets/*.conf`).

Legenda stato:

| Stato | Significato |
|-------|-------------|
| **Mappato** | Importato e applicato dal DSP GearCraft (WirePlumber filter-chain) |
| **Parziale** | Approssimato (parametri ridotti o effetto semplificato) |
| **Nota** | Riconosciuto; solo annotato nel campo `notes` del preset |
| **Ignorato** | Non letto (UI-only / non audio / non supportato) |

---

## 1. Master / output

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `master_enable` | bool | `AudioEffects.bypass = !master_enable` | **Mappato** | Se assente, si assume ON |
| `master_limthreshold` | float dB | `limiter_db` | **Mappato** | Default JDSP ≈ `-0.1` |
| `master_limrelease` | float ms | — | **Nota** | Release limiter non esposto; usiamo costante DSP |
| `master_postgain` | float dB | `volume_boost_db` | **Mappato** | Clamp −12…+12 dB |

---

## 2. Equalizer (Tone / multimodal)

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `tone_enable` | bool | `eq_enabled` | **Mappato** | |
| `tone_eq` | string | `eq_bands[]` | **Mappato** | Vedi formato sotto |
| `tone_filtertype` | int | — | **Ignorato** | FIR/IIR order non selezionabile in GearCraft |
| `tone_interpolation` | int | — | **Ignorato** | |

### Formato `tone_eq`

```
freq1;freq2;…;freqN;gain1;gain2;…;gainN
```

- Prima metà: centri in Hz  
- Seconda metà: gain in dB  
- Esempio (4 bande): `25;40;63;100;1.0;2.0;3.0;4.0`  
- Gain clamp in import: **−12…+12 dB**

### GraphicEQ (EqualizerAPO-style)

| Chiave | Tipo | GearCraft | Stato |
|--------|------|-----------|-------|
| `graphiceq_enable` | bool | contribuisce a `eq_enabled` | **Mappato** |
| `graphiceq_param` | string | `eq_bands[]` | **Mappato** |

Formato accettato:

```
GraphicEQ: 25 0; 40 1.5; 63 -1; …
```

oppure senza prefisso: `25 0; 40 1.5; …`

Se sia `tone_eq` sia `graphiceq_param` sono presenti, **prevale `tone_eq`**.

---

## 3. Bass (Dynamic bass boost)

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `bass_enable` | bool | `bass_boost` | **Mappato** | Bass dinamico → low-shelf fisso |
| `bass_maxgain` | float dB | `bass_boost_db` | **Mappato** | Clamp 0…12; **non** c’è auto bandwidth/cutoff |

**Non mappati (dinamici JDSP):** stima automatica di gain/bandwidth/cutoff dallo stream → in GearCraft il bass è **statico** (shelf ~80 Hz in `dsp_pw`).

---

## 4. Crossfeed

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `crossfeed_enable` | bool | `crossfeed` | **Mappato** | |
| `crossfeed_mode` | int | — | **Parziale** | Solo path BS2B/soft; mode 0/1 non distinti |
| `crossfeed_bs2b_fcut` | float Hz | — | **Nota** | Cutoff BS2B fisso lato LADSPA se presente |
| `crossfeed_bs2b_feed` | float | `crossfeed_strength` | **Mappato** | Formula: `strength = clamp(feed / 1.5, 0, 100)` |

Esempio: `feed=60` → `crossfeed_strength ≈ 40`.

---

## 5. Compander / compressione

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `compander_enable` | bool | `compressor` | **Parziale** | Multiband → mono compressor LADSPA `sc4` |
| `compander_granularity` | int | — | **Ignorato** | |
| `compander_response` | string | — | **Ignorato** | Curve per banda non applicate |
| `compander_timeconstant` | float | — | **Ignorato** | Attack/release fissi in DSP |
| `compander_time_freq_transforms` | int | — | **Ignorato** | |

`compressor_strength` all’import: **50** se enable, altrimenti **0**.

---

## 6. Stereo wideness

| Chiave JamesDSP | Tipo | GearCraft | Stato | Note |
|-----------------|------|-----------|-------|------|
| `stereowide_enable` | bool | `wideness` | **Mappato** | |
| `stereowide_level` | 0–100 | `wideness_pct` | **Mappato** | Multiband JDSP → wideness globale |

---

## 7. Reverb (Progenitor 2)

| Chiave JamesDSP | Tipo | GearCraft | Stato |
|-----------------|------|-----------|-------|
| `reverb_enable` | bool | `reverb = true` | **Parziale** (flag only) |
| `reverb_osf` | int | — | **Nota** |
| `reverb_bassboost` | float | — | **Nota** |
| `reverb_decay` | float | — | **Nota** |
| `reverb_delay` | float | — | **Nota** |
| `reverb_wet` / `reverb_finalwet` / `reverb_finaldry` | float | — | **Nota** |
| `reverb_width` | float | — | **Nota** |
| `reverb_lfo_spin` / `reverb_lfo_wander` | float | — | **Nota** |
| `reverb_lpf_*` | float | — | **Nota** |
| `reverb_reflection_*` | float | — | **Nota** |

GearCraft non esegue la rete IIR Progenitor; il flag serve da reminder in UI/notes.

---

## 8. Convolver

| Chiave JamesDSP | Tipo | GearCraft | Stato |
|-----------------|------|-----------|-------|
| `convolver_enable` | bool | notes | **Nota** |
| `convolver_file` | path | notes | **Nota** |
| `convolver_optimization_mode` | int | — | **Ignorato** |
| `convolver_waveform_edit` | string | — | **Ignorato** |

IR (mono/stereo/true-stereo) non caricate. Usare EasyEffects/JDSP se serve convoluzione.

---

## 9. ViPER-DDC

| Chiave JamesDSP | Tipo | GearCraft | Stato |
|-----------------|------|-----------|-------|
| `ddc_enable` | bool | notes | **Nota** |
| `ddc_file` | path `.vdc` | notes | **Nota** |

---

## 10. Liveprog (EEL2)

| Chiave JamesDSP | Tipo | GearCraft | Stato |
|-----------------|------|-----------|-------|
| `liveprog_enable` | bool | notes | **Nota** |
| `liveprog_file` | path | notes | **Nota** |

Nessuna VM EEL2 in GearCraft.

---

## 11. Tube / analog modelling

| Chiave JamesDSP | Tipo | GearCraft | Stato |
|-----------------|------|-----------|-------|
| `tube_enable` | bool | notes | **Nota** |
| `tube_pregain` | float | notes | **Nota** |

---

## 12. Parametri non presenti in GearCraft

Effetti JDSP senza controparte strutturale:

- AutoEQ database (online/offline)
- Frequency-detecting dynamic bass
- Multiband dynamic range **compander** completo
- Partitioned convolver
- Scripting Liveprog
- Profili per-device automatici JDSP (GearCraft ha regole preset/game separate)

---

## 13. Pipeline DSP risultante (dopo import + Apply)

Ordine tipico in `dsp_pw` (semplificato):

```
preamp/volume_boost → EQ bands → bass shelf → spatial (se presente)
  → lowpass (se on) → compressor sc4 → crossfeed BS2B/soft → limiter
```

Campi importati che **non** attivano nodi finché non supportati dal graph: `reverb`.

---

## 14. Esempio di conversione

**Ingresso** (`Clear-like.conf`):

```ini
master_enable=true
master_limthreshold=-0.1
master_postgain=0
tone_enable=true
tone_eq=25;40;63;100;160;250;400;630;1000;1600;2500;4000;6300;10000;16000;1.5;3.5;4.5;6.5;5.5;4.0;1.5;1.0;0.5;3.0;4.5;6.0;6.5;7.0;6.0
bass_enable=true
bass_maxgain=5
crossfeed_enable=false
compander_enable=false
stereowide_enable=false
reverb_enable=false
convolver_enable=false
ddc_enable=false
liveprog_enable=false
tube_enable=false
```

**Uscita GearCraft (concettuale):**

```json
{
  "id": "jdsp-clear-like",
  "name": "Clear-like",
  "effects": {
    "eq_enabled": true,
    "eq_bands": [
      { "freq_hz": 25, "gain_db": 1.5 },
      { "freq_hz": 40, "gain_db": 3.5 }
    ],
    "bass_boost": true,
    "bass_boost_db": 5.0,
    "crossfeed": false,
    "compressor": false,
    "limiter": true,
    "limiter_db": -0.1,
    "volume_boost_db": 0.0,
    "bypass": false
  },
  "notes": "Imported from JamesDSP .conf"
}
```

---

## 15. API import

Vedi anche `docs/JAMESDSP_IMPORT.md`.

| Comando IPC | Uso |
|-------------|-----|
| `ImportJamesDspPreset { path, apply }` | Un file `.conf` |
| `ImportJamesDspPreset { content, name, apply }` | Corpo inline |
| `ImportJamesDspDir { path }` | Tutti i `.conf` della cartella |

Codice: `crates/gearcraft-daemon/src/jamesdsp_import.rs` → `conf_to_preset()`.
