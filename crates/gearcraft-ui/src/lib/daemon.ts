/**
 * Thin client for GearCraft daemon via Tauri invoke.
 */

import { invoke } from "@tauri-apps/api/core";
import type { DeviceInfo, DeviceStatus } from "./types";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type DaemonError = {
  kind: "offline" | "invoke" | "unknown";
  message: string;
};

export interface MacroAction {
  key: string;
  delay_ms?: number;
}

export interface MacroDef {
  id: string;
  name: string;
  device_id?: string | null;
  trigger: string;
  actions: MacroAction[];
  enabled?: boolean;
}

export interface ChatMixState {
  balance: number;
  game_volume: number;
  chat_volume: number;
  backend?: string;
  available?: boolean;
  applied?: boolean;
  message?: string;
  game_sink?: string | null;
  chat_sink?: string | null;
}

export async function pingDaemon(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    const res = await invoke<string>("ping_daemon");
    return res === "pong" || res != null;
  } catch {
    return false;
  }
}

export async function fetchDevices(): Promise<{
  devices: DeviceInfo[];
  error: DaemonError | null;
}> {
  if (!isTauri()) {
    return {
      devices: [],
      error: {
        kind: "offline",
        message: "Not running inside Tauri – start with `npm run tauri dev`",
      },
    };
  }

  try {
    const data = await invoke<DeviceInfo[]>("get_devices");
    return { devices: Array.isArray(data) ? data : [], error: null };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    const offline =
      message.includes("cannot connect") ||
      message.includes("No such file") ||
      message.includes("Connection refused");
    return {
      devices: [],
      error: {
        kind: offline ? "offline" : "invoke",
        message: offline
          ? "Daemon offline – run `cargo run -p gearcraft-daemon`"
          : message,
      },
    };
  }
}

export async function fetchDeviceStatus(
  deviceId: string,
): Promise<DeviceStatus | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<DeviceStatus>("get_device_status", {
      device_id: deviceId,
    });
  } catch {
    return null;
  }
}

export async function setDeviceColor(
  deviceId: string,
  zone: string,
  r: number,
  g: number,
  b: number,
): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await invoke("set_color", {
      device_id: deviceId,
      zone,
      r,
      g,
      b,
    });
    return true;
  } catch {
    return false;
  }
}

export async function listMacros(): Promise<MacroDef[]> {
  if (!isTauri()) return [];
  try {
    const data = await invoke<MacroDef[]>("list_macros");
    return Array.isArray(data) ? data : [];
  } catch {
    return [];
  }
}

export async function runMacro(macroId: string): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await invoke("run_macro", { macro_id: macroId });
    return true;
  } catch {
    return false;
  }
}

export async function deleteMacro(macroId: string): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await invoke("delete_macro", { macro_id: macroId });
    return true;
  } catch {
    return false;
  }
}

export async function upsertMacro(macroDef: MacroDef): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await invoke("upsert_macro", { macro_def: macroDef });
    return true;
  } catch {
    return false;
  }
}

export async function getChatMix(): Promise<ChatMixState | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<ChatMixState>("get_chatmix");
  } catch {
    return null;
  }
}

export async function setChatMix(balance: number): Promise<ChatMixState | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<ChatMixState>("set_chatmix", {
      balance: Math.round(balance),
    });
  } catch {
    return null;
  }
}

export async function setupChatMix(): Promise<ChatMixState & { ok?: boolean } | null> {
  if (!isTauri()) return null;
  try {
    return await invoke("setup_chatmix");
  } catch {
    return null;
  }
}
