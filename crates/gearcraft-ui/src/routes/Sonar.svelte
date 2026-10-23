<script lang="ts">
  import { onMount } from "svelte";
  import type { DeviceInfo, DeviceStatus } from "../lib/types";
  import {
    fetchDevices,
    fetchDeviceStatus,
    getChatMix,
    setChatMix,
    setupChatMix,
    type ChatMixState,
  } from "../lib/daemon";

  let headsets = $state<DeviceInfo[]>([]);
  let statuses = $state<Record<string, DeviceStatus | null>>({});
  let error = $state<string | null>(null);
  let loading = $state(true);
  let chatmix = $state<ChatMixState>({
    balance: 0,
    game_volume: 100,
    chat_volume: 100,
  });
  let chatmixBusy = $state(false);
  let setupMsg = $state<string | null>(null);

  async function load() {
    loading = true;
    const result = await fetchDevices();
    error = result.error?.message ?? null;
    headsets = result.devices.filter((d) => d.kind === "headset");
    const map: Record<string, DeviceStatus | null> = {};
    for (const h of headsets) {
      map[h.id] = await fetchDeviceStatus(h.id);
    }
    statuses = map;
    const cm = await getChatMix();
    if (cm) chatmix = cm;
    loading = false;
  }

  async function onBalance(v: number) {
    chatmixBusy = true;
    const next = await setChatMix(v);
    if (next) {
      chatmix = { ...chatmix, ...next };
    } else {
      const b = Math.round(v);
      chatmix = {
        ...chatmix,
        balance: b,
        game_volume: Math.round((100 + b) / 2),
        chat_volume: Math.round((100 - b) / 2),
        applied: false,
        message: "Daemon offline",
      };
    }
    chatmixBusy = false;
  }

  async function onSetup() {
    setupMsg = "Creating sinks…";
    const res = await setupChatMix();
    if (res) {
      setupMsg = res.message ?? "Done";
      const cm = await getChatMix();
      if (cm) chatmix = cm;
    } else {
      setupMsg = "Setup failed (need pactl / pipewire-pulse)";
    }
  }

  onMount(() => {
    load();
    const id = setInterval(load, 10000);
    return () => clearInterval(id);
  });
</script>

<div class="p-8 h-full overflow-y-auto">
  <div class="mb-6">
    <h1 class="text-2xl font-bold text-gc-text tracking-tight">Sonar</h1>
    <p class="text-sm text-gc-text-sec mt-1">
      Headset status, battery and PipeWire ChatMix.
    </p>
  </div>

  {#if error}
    <div class="mb-6 rounded-xl border border-gc-error/40 bg-gc-error/10 px-4 py-3 text-sm text-gc-error">
      {error}
    </div>
  {/if}

  <section class="mb-8 max-w-2xl bg-gc-card border border-gc-border rounded-xl p-5">
    <div class="flex items-start justify-between gap-3 mb-1">
      <h2 class="text-sm font-semibold uppercase tracking-wider">ChatMix</h2>
      <span
        class="text-[10px] px-2 py-0.5 rounded-full border
          {chatmix.available
          ? 'border-gc-accent/40 text-gc-accent'
          : 'border-gc-border text-gc-text-sec'}"
      >
        {chatmix.backend ?? "—"} · {chatmix.available ? "ready" : "not linked"}
      </span>
    </div>
    <p class="text-xs text-gc-text-sec mb-4">
      Balance game vs chat sinks on PipeWire. Create virtual sinks, then route apps in pavucontrol /
      Helvum.
    </p>

    <div class="flex items-center justify-between text-xs text-gc-text-sec mb-2">
      <span>Chat</span>
      <span class="text-gc-text font-mono">{chatmix.balance}</span>
      <span>Game</span>
    </div>
    <input
      type="range"
      min="-100"
      max="100"
      step="1"
      value={chatmix.balance}
      class="w-full accent-gc-accent"
      disabled={chatmixBusy}
      oninput={(e) => onBalance(+e.currentTarget.value)}
    />
    <div class="mt-4 grid grid-cols-2 gap-3">
      <div class="rounded-lg bg-gc-bg border border-gc-border px-3 py-2">
        <div class="text-[10px] uppercase text-gc-text-sec tracking-wider">Chat</div>
        <div class="text-lg font-semibold text-gc-text tabular-nums">{chatmix.chat_volume}%</div>
      </div>
      <div class="rounded-lg bg-gc-bg border border-gc-border px-3 py-2">
        <div class="text-[10px] uppercase text-gc-text-sec tracking-wider">Game</div>
        <div class="text-lg font-semibold text-gc-accent tabular-nums">{chatmix.game_volume}%</div>
      </div>
    </div>

    {#if chatmix.message}
      <p class="mt-3 text-[11px] text-gc-text-sec/80 break-words">{chatmix.message}</p>
    {/if}

    <div class="mt-4 flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="text-xs px-3 py-1.5 rounded-lg border border-gc-border hover:border-gc-accent/50
               text-gc-text-sec hover:text-gc-accent transition-colors"
        onclick={onSetup}
      >
        Setup Game/Chat sinks
      </button>
      {#if setupMsg}
        <span class="text-[11px] text-gc-accent">{setupMsg}</span>
      {/if}
    </div>
  </section>

  {#if loading && headsets.length === 0}
    <div class="h-32 rounded-xl bg-gc-card border border-gc-border animate-pulse max-w-2xl"></div>
  {:else if headsets.length === 0}
    <div
      class="rounded-xl border border-dashed border-gc-border bg-gc-card/50 px-6 py-12 text-center max-w-2xl"
    >
      <p class="text-gc-text-sec text-sm">No headset detected.</p>
      <p class="text-xs text-gc-text-sec/70 mt-1">Connect an Arctis device or start the daemon.</p>
    </div>
  {:else}
    <div class="grid gap-4 max-w-2xl">
      {#each headsets as h}
        {@const st = statuses[h.id]}
        <div class="bg-gc-card border border-gc-border rounded-xl p-5 flex items-center gap-4">
          <div class="w-14 h-14 rounded-xl bg-gc-accent/15 flex items-center justify-center text-2xl">
            🎧
          </div>
          <div class="flex-1 min-w-0">
            <div class="font-semibold text-gc-text">{h.name}</div>
            <div class="text-xs text-gc-text-sec">{h.vendor} · {h.id}</div>
          </div>
          <div class="text-right">
            {#if st?.battery != null}
              <div class="text-2xl font-bold text-gc-accent tabular-nums">{st.battery}%</div>
              <div class="text-[10px] text-gc-text-sec uppercase tracking-wider">Battery</div>
            {:else}
              <div class="text-sm text-gc-text-sec">—</div>
            {/if}
            <div class="mt-1 flex items-center justify-end gap-1.5">
              <span
                class="w-1.5 h-1.5 rounded-full
                  {st?.status === 'online' ? 'bg-gc-online' : 'bg-gc-offline'}"
              ></span>
              <span class="text-xs capitalize text-gc-text-sec">{st?.status ?? "unknown"}</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
