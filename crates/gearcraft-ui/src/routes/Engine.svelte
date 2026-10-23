<script lang="ts">
  import { onMount } from "svelte";
  import ColorPicker from "../components/ColorPicker.svelte";
  import type { DeviceInfo } from "../lib/types";
  import {
    fetchDevices,
    setDeviceColor,
    listMacros,
    runMacro,
    deleteMacro,
    upsertMacro,
    type MacroDef,
  } from "../lib/daemon";
  import { getSelectedDevice, setSelectedDevice } from "../lib/store";

  const ZONES = [
    { id: "all", label: "All zones" },
    { id: "logo", label: "Logo" },
    { id: "z1", label: "Strip top" },
    { id: "z2", label: "Strip middle" },
    { id: "z3", label: "Strip bottom" },
  ] as const;

  let devices = $state<DeviceInfo[]>([]);
  let selectedId = $state<string>("");
  let zone = $state<string>("logo");
  let r = $state(0);
  let g = $state(231);
  let b = $state(160);
  let applying = $state(false);
  let message = $state<{ ok: boolean; text: string } | null>(null);
  let loadError = $state<string | null>(null);

  let macros = $state<MacroDef[]>([]);
  let macroMsg = $state<string | null>(null);
  let newName = $state("");
  let newTrigger = $state("Mouse button 4");
  let newKey = $state("F8");

  const selected = $derived(devices.find((d) => d.id === selectedId) ?? null);

  async function loadDevices() {
    const result = await fetchDevices();
    devices = result.devices;
    loadError = result.error?.message ?? null;
    const fromStore = getSelectedDevice();
    if (fromStore && devices.some((d) => d.id === fromStore.id)) {
      selectedId = fromStore.id;
    } else if (devices.length && !selectedId) {
      selectedId = devices[0].id;
    }
  }

  async function loadMacros() {
    macros = await listMacros();
  }

  async function apply() {
    if (!selectedId) return;
    applying = true;
    message = null;
    const ok = await setDeviceColor(selectedId, zone, r, g, b);
    message = ok
      ? { ok: true, text: `Applied rgb(${r}, ${g}, ${b}) → ${zone}` }
      : {
          ok: false,
          text: "Failed to set colour (daemon offline or device error)",
        };
    applying = false;
  }

  function onSelectDevice(id: string) {
    selectedId = id;
    const d = devices.find((x) => x.id === id);
    if (d) setSelectedDevice(d);
  }

  async function onRun(id: string) {
    const ok = await runMacro(id);
    macroMsg = ok ? `Ran macro ${id}` : "Failed to run macro";
    setTimeout(() => (macroMsg = null), 2500);
  }

  async function onDelete(id: string) {
    if (!(await deleteMacro(id))) {
      macroMsg = "Delete failed";
      return;
    }
    await loadMacros();
  }

  async function onAdd() {
    if (!newName.trim() || !newKey.trim()) return;
    const def: MacroDef = {
      id: `user-${Date.now()}`,
      name: newName.trim(),
      trigger: newTrigger.trim() || "Custom",
      actions: [{ key: newKey.trim(), delay_ms: 0 }],
      enabled: true,
      device_id: selectedId || null,
    };
    if (await upsertMacro(def)) {
      newName = "";
      newKey = "F8";
      await loadMacros();
      macroMsg = "Macro saved";
    } else {
      macroMsg = "Save failed (daemon offline?)";
    }
  }

  onMount(() => {
    loadDevices();
    loadMacros();
  });
</script>

<div class="p-8 h-full overflow-y-auto">
  <div class="mb-6">
    <h1 class="text-2xl font-bold text-gc-text tracking-tight">Engine</h1>
    <p class="text-sm text-gc-text-sec mt-1">
      RGB lighting, macros and performance settings.
    </p>
  </div>

  {#if loadError}
    <div
      class="mb-6 rounded-xl border border-gc-error/40 bg-gc-error/10 px-4 py-3 text-sm text-gc-error"
    >
      {loadError}
    </div>
  {/if}

  <div class="grid gap-6 lg:grid-cols-2 max-w-4xl">
    <section class="bg-gc-card border border-gc-border rounded-xl p-5 space-y-4">
      <h2 class="text-sm font-semibold text-gc-text uppercase tracking-wider">Device</h2>
      {#if devices.length === 0}
        <p class="text-sm text-gc-text-sec">No devices available.</p>
      {:else}
        <select
          class="w-full bg-gc-bg border border-gc-border rounded-lg px-3 py-2 text-sm
                 text-gc-text focus:outline-none focus:border-gc-accent/50"
          value={selectedId}
          onchange={(e) => onSelectDevice(e.currentTarget.value)}
        >
          {#each devices as d}
            <option value={d.id}>{d.name} — {d.vendor}</option>
          {/each}
        </select>
      {/if}
      <div>
        <div class="text-xs text-gc-text-sec mb-2">Zone</div>
        <div class="flex flex-wrap gap-2">
          {#each ZONES as z}
            <button
              type="button"
              class="px-3 py-1.5 rounded-lg text-xs border transition-colors
                {zone === z.id
                ? 'border-gc-accent bg-gc-accent/15 text-gc-accent'
                : 'border-gc-border text-gc-text-sec hover:border-gc-accent/40'}"
              onclick={() => (zone = z.id)}
            >
              {z.label}
            </button>
          {/each}
        </div>
      </div>
      {#if selected}
        <div class="text-xs text-gc-text-sec pt-2 border-t border-gc-border">
          <span class="font-mono">{selected.id}</span>
          · PID 0x{selected.product_id.toString(16).padStart(4, "0")}
        </div>
      {/if}
    </section>

    <section class="bg-gc-card border border-gc-border rounded-xl p-5 space-y-4">
      <h2 class="text-sm font-semibold text-gc-text uppercase tracking-wider">Colour</h2>
      <ColorPicker
        {r}
        {g}
        {b}
        onChange={(nr, ng, nb) => {
          r = nr;
          g = ng;
          b = nb;
        }}
      />
      <button
        type="button"
        class="w-full py-2.5 rounded-lg font-medium text-sm transition-colors
               bg-gc-accent text-gc-bg hover:bg-gc-accent-hover disabled:opacity-50"
        disabled={!selectedId || applying}
        onclick={apply}
      >
        {applying ? "Applying…" : "Apply to device"}
      </button>
      {#if message}
        <div
          class="text-xs rounded-lg px-3 py-2
            {message.ok ? 'bg-gc-accent/10 text-gc-accent' : 'bg-gc-error/10 text-gc-error'}"
        >
          {message.text}
        </div>
      {/if}
    </section>
  </div>

  <section class="mt-8 max-w-4xl bg-gc-card border border-gc-border rounded-xl p-5">
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-sm font-semibold text-gc-text uppercase tracking-wider">Macros</h2>
      {#if macroMsg}
        <span class="text-xs text-gc-accent">{macroMsg}</span>
      {/if}
    </div>

    <div class="space-y-2 mb-4">
      {#each macros as m (m.id)}
        <div
          class="flex items-center gap-3 rounded-lg border border-gc-border bg-gc-bg/50 px-3 py-2"
        >
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium truncate">{m.name}</div>
            <div class="text-[11px] text-gc-text-sec truncate">
              {m.trigger} → {m.actions.map((a) => a.key).join(", ")}
            </div>
          </div>
          <button
            type="button"
            class="text-xs px-2.5 py-1 rounded-lg border border-gc-border hover:border-gc-accent/50 text-gc-text-sec hover:text-gc-accent"
            onclick={() => onRun(m.id)}
          >
            Test
          </button>
          <button
            type="button"
            class="text-xs px-2.5 py-1 rounded-lg border border-gc-border hover:border-gc-error/50 text-gc-text-sec hover:text-gc-error"
            onclick={() => onDelete(m.id)}
          >
            Delete
          </button>
        </div>
      {:else}
        <p class="text-sm text-gc-text-sec">No macros yet.</p>
      {/each}
    </div>

    <div class="grid gap-2 sm:grid-cols-4 items-end border-t border-gc-border pt-4">
      <label class="text-xs text-gc-text-sec sm:col-span-1">
        Name
        <input
          class="mt-1 w-full bg-gc-bg border border-gc-border rounded-lg px-2 py-1.5 text-sm text-gc-text"
          bind:value={newName}
          placeholder="My macro"
        />
      </label>
      <label class="text-xs text-gc-text-sec">
        Trigger
        <input
          class="mt-1 w-full bg-gc-bg border border-gc-border rounded-lg px-2 py-1.5 text-sm text-gc-text"
          bind:value={newTrigger}
        />
      </label>
      <label class="text-xs text-gc-text-sec">
        Key sequence
        <input
          class="mt-1 w-full bg-gc-bg border border-gc-border rounded-lg px-2 py-1.5 text-sm text-gc-text"
          bind:value={newKey}
          placeholder="ctrl+c"
        />
      </label>
      <button
        type="button"
        class="py-1.5 rounded-lg text-sm font-medium bg-gc-accent/20 text-gc-accent hover:bg-gc-accent/30"
        onclick={onAdd}
      >
        Add macro
      </button>
    </div>
    <p class="mt-3 text-[11px] text-gc-text-sec/70">
      Execution uses a uinput when /dev/uinput is available (group input); otherwise keys are logged.
    </p>
  </section>
</div>
