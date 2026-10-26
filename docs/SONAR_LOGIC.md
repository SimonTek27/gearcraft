# Analisi della logica di SteelSeries Sonar (riferimento per GearCraft)

> Sorgenti: articolo ufficiale SteelSeries Support "Setting up Sonar for Streaming",
> pagina prodotto steelseries.com/gg/sonar e /gg/sonar-for-streamers,
> capitoli dei tutorial ufficiali (Mixer Tab, Streamer Mode, Game/Chat/Media/Aux/Mic tabs).
> Nota: il video YouTube citato (HYvGI0sNhsg) non è leggibile frame-by-frame da questo ambiente;
> l'analisi si basa sulle fonti ufficiali che documentano la stessa logica.

---

## 1. Modello a due mix: Personal e Stream

Sonar si fonda su **due mix separati**:

| Mix | Icona | Destinazione |
|-----|-------|--------------|
| **Personal** | 🎧 cuffia | headset / speaker dell'utente |
| **Stream** | 📡 streaming | ingresso audio del software di streaming (OBS, ecc.) — "Sonar Stream" |

- Lo streaming software vede **un solo sink virtuale**: "Sonar Stream".
- Il fader di un canale controlla **lo stesso contenuto** su entrambi i mix, ma con **volumi separati** (personal vs stream) quando la Streamer Mode è ON.

## 2. Master

Il canale **Master** contiene il mix di tutti gli altri canali (game, chat, media, aux, mic):

- Due livelli: **Personal** ed **Stream** (sliders indipendenti).
- Puoi mutare il personal mix e/o lo stream mix.
- Icona **orecchio** (Ear): sentire in anteprima esattamente quello che sentirà il pubblico ("monitor").
- Output personal = headset/speaker dell'utente; output stream = "Sonar Stream" (per OBS).

## 3. Canali

Canali disponibles e uso tipico:

| Canale | Tipo | Uso |
|--------|------|-----|
| **Game** | output | suoni del gioco ("pew pew") |
| **Chat** | output | Discord, Teams, chiamate |
| **Media** | output | musica, video (Spotify, ecc.) |
| **Aux** | output | tutto il resto (browser, notifiche, alerts) |
| **Mic** | input | microfono dell'utente (solo **un** mic lascia stream) |
| **Master** | mix | somma di tutti i canali |

Ogni canale ha un pannello settings (icona ingranaggio) con:

1. **Includi nel personal mix**: on/off.
2. **Includi nel stream mix**: on/off (indipendente da #1 → permettono
   "sento ma il pubblico non sente" e viceversa).
3. Volume per mix con mute separato.
4. Routing manuale (App to device routing).
5. Shortcut per canale per personal e per stream separatamente.

## 4. Tab per canale (pattern UI)

Ogni canale ha una scheda dedicata con preset + effetti:

- **Game**: preset per titolo / spatial, EQ.
- **Chat**: ClearCast AI Noise Cancellation.
- **Media / Aux**: EQ e preset dedicati.
- **Mic**: EQ 10-band, Noise Gate, Compressor, gain.

## 5. Mixer tab

- Fader per canale (incl. master).
- Permette di **nascondere/disabilitare Media o Aux** (e reinserirle con "Add channel").
- **ChatMix slider** in basso: bilancia game↔chat (priorità audio).

## 6. App routing

- Ogni app → assegnata a un canale (drag&drop) o via Windows Volume Mixer.
- Le app "smistate" appaiono associate al canale, non a device fisici.

---

## 7. Confronto con l'attuale UI GearCraft

| Aspetto Sonar | GearCraft oggi | Note / gap |
|---|---|---|
| 2 mix (personal/stream) | due fader per canale streamer-mode | ✔ coerente |
| Master: personal + stream | master: **un solo fader** | ⚠ divergenza dalla logica Sonar — decidere |
| Master ear icon (monitor) | assente | gap opzionale |
| Includi in personal/stream per canale (cog) | fader + `preset-on` | differenza semantica |
| Mute personale e stream separati | singolo "M" per canale | gap |
| ChatMix in basso | presente | ✔ |
| Nascondi Media/Aux | assente | opzionale |
| Preset per canale con effetti | pagina Preset/FX (EQ, spatial, mic) | ✔ |
| App routing/riquadro app | box "App" per canale | dati da daemon mancanti |
| Stream out = sink unico | `gearcraft.mic`/sinks PipeWire | lato daemon |

## 8. Decisione da prendere

1. **Master con due fader** quando streamer-mode è ON (personal + stream), come da logica Sonar ufficiale → raccomandato per coerenza col video.
2. Le "due voci per canale" veritiere di Sonar non sono "2 uscite diverse" ma **lo stesso segnale con volume personal e stream separati**:
   → implementare `personal-volume` / `stream-volume` per il canale (in luogo di `volume` / `streamer-volume`).
3. Mute separato: introdurre `mute-personal` / `mute-stream` (o fader al minimo).
4. Toggle "includi in stream"/"includi in personal" per canale (sostituisce/affianca `preset-on`).
