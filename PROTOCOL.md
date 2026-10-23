# GearCraft IPC Protocol

Unix domain socket: `$XDG_RUNTIME_DIR/gearcraft.sock` (fallback `/tmp/gearcraft.sock`).

Framing: `[u32 LE length][UTF-8 JSON]`.

Commands include: `ping`, `get_devices`, `get_device_status`, `apply_profile`, `set_color`, `list_addons`, `list_macros`, `upsert_macro`, `delete_macro`, `run_macro`, `get_chatmix`, `set_chatmix`, `setup_chatmix`, `shutdown`.

See in-repo PROTOCOL.md for full schema and examples.
