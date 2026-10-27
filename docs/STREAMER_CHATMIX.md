# Streamer mode & ChatMix (SteelSeries-aligned)

## ChatMix
- Controls **only Game ↔ Chat** balance (not Master/Media/Aux/Mic).
- UI: vertical strip **between** the GAME and CHAT channel columns.
- Balance −100 … +100 maps to game/chat virtual sink levels (PipeWire).

## Streamer mode
Matches SteelSeries Sonar dual-path idea:

| Path | Label | Role |
|------|-------|------|
| Personal | 🎧 YOU | What **you** hear (monitoring) |
| Stream | 📡 STREAM | What goes to **broadcast** / VOIP stream mix |

Each channel strip shows:
- One real mixer fader (YOU) always
- Second fader (STREAM) when streamer mode is on
- Independent levels (`volume` vs `streamer-volume`)

## Fader look
Physical-style cap, groove, scale ticks, **two LED meter columns** (signal + peak).
