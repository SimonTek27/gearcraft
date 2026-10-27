# app.slint packaging

Full UI is stored as `app.slint.gz.b64` (gzip + base64).

```bash
bash scripts/restore-app-slint.sh
# or:
base64 -d crates/gearcraft-ui/ui/app.slint.gz.b64 | gzip -dc > crates/gearcraft-ui/ui/app.slint
```
