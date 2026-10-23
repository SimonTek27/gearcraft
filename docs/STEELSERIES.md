# SteelSeries protocols

VID `0x1038`.

- Rival 3: colour `05 00 <LED> <R> <G> <B> <BR>`
- Apex: simplified `21 FF` + RGB zones
- Arctis: battery request `06 18`

Build with live USB: `cargo run -p gearcraft-daemon --features hidapi`
