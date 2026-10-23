<script lang="ts">
  import { onMount } from "svelte";
  import DeviceCard from "../components/DeviceCard.svelte";
  import type { ConnectionStatus, DeviceInfo } from "../lib/types";
  import { fetchDevices, type DaemonError } from "../lib/daemon";
  import { setSelectedDevice, requestNavigate } from "../lib/store";

  function openDevice(device: DeviceInfo) {
    setSelectedDevice(device);
    requestNavigate("engine");
  }

  let devices = $state<DeviceInfo[]>([]);
  let statusMap = $state<Record<string, ConnectionStatus>>({});
  let loading = $state(true);
  let error = $state<DaemonError | null>(null);
  let lastRefresh = $state<Date | null>(null);

  async function load() {
    loading = true;
    error = null;
    const result = await fetchDevices();
    devices = result.devices;
    error = result.error;
    // Assume online when listed by daemon (status endpoint can refine later)
    const map: Record<string, ConnectionStatus> = {};
    for (const d of devices) {
      map[d.id] = "online";
    }
    statusMap = map;
    lastRefresh = new Date();
    loading = false;
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 8000);
    return () => clearInterval(interval);
  });
</script>

<div class="p-8 h-full overflow-y-auto">
  <div class="mb-6 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold text-gc-text tracking-tight">My Devices</h1>
      <p class="text-sm text-gc-text-sec mt-1">
        Manage lighting, macros and profiles for your peripherals.
      </p>
    </div>
    <button
      class="shrink-0 px-3 py-1.5 text-xs font-medium rounded-lg border border-gc-border
             text-gc-text-sec hover:text-gc-text hover:border-gc-accent/40 hover:bg-gc-hover
             transition-colors disabled:opacity-50"
      onclick={load}
      disabled={loading}
      title="Refresh devices"
    >
      {loading ? "Loading…" : "Refresh"}
    </button>
  </div>

  {#if error}
    <div
      class="mb-6 flex items-start gap-3 rounded-xl border border-gc-error/40 bg-gc-error/10 px-4 py-3"
    >
      <span class="mt-0.5 text-gc-error text-lg leading-none">●</span>
      <div class="flex-1 min-w-0">
        <div class="text-sm font-medium text-gc-error">
          {error.kind === "offline" ? "Daemon offline" : "Connection error"}
        </div>
        <div class="text-xs text-gc-text-sec mt-0.5 break-words">{error.message}</div>
        {#if error.kind === "offline"}
          <pre
            class="mt-2 text-[11px] text-gc-text-sec bg-gc-bg/60 rounded-lg px-3 py-2 overflow-x-auto"
          >cargo run -p gearcraft-daemon</pre>
        {/if}
      </div>
      <button
        class="text-xs text-gc-accent hover:text-gc-accent-hover shrink-0"
        onclick={load}
      >
        Retry
      </button>
    </div>
  {/if}

  {#if loading && devices.length === 0 && !error}
    <div class="flex flex-wrap gap-5">
      {#each [1, 2, 3] as _}
        <div
          class="w-[180px] min-h-[200px] rounded-xl bg-gc-card border border-gc-border animate-pulse"
        ></div>
      {/each}
    </div>
  {:else if devices.length === 0 && !error}
    <div
      class="rounded-xl border border-dashed border-gc-border bg-gc-card/50 px-6 py-12 text-center"
    >
      <p class="text-gc-text-sec text-sm">No devices detected.</p>
      <p class="text-xs text-gc-text-sec/70 mt-1">
        Connect a supported peripheral or check the daemon logs.
      </p>
    </div>
  {:else}
    <div class="flex flex-wrap gap-5">
      {#each devices as device (device.id)}
        <DeviceCard
          {device}
          status={statusMap[device.id] ?? "offline"}
          onclick={() => openDevice(device)}
        />
      {/each}

      <button
        class="flex flex-col items-center justify-center w-[180px] min-h-[200px]
               border-2 border-dashed border-gc-border rounded-xl
               text-gc-text-sec hover:border-gc-accent/50 hover:text-gc-accent
               transition-colors"
      >
        <svg
          width="28"
          height="28"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
        >
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
        <span class="text-sm mt-2">Add Device</span>
      </button>
    </div>
  {/if}

  {#if lastRefresh}
    <p class="mt-6 text-[11px] text-gc-text-sec/60">
      Last refresh: {lastRefresh.toLocaleTimeString()}
    </p>
  {/if}
</div>
