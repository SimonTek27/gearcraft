<script lang="ts">
  interface Props {
    r: number;
    g: number;
    b: number;
    onChange: (r: number, g: number, b: number) => void;
  }
  let { r, g, b, onChange }: Props = $props();

  const presets: { name: string; r: number; g: number; b: number }[] = [
    { name: "Accent", r: 0, g: 231, b: 160 },
    { name: "White", r: 255, g: 255, b: 255 },
    { name: "Red", r: 255, g: 40, b: 40 },
    { name: "Blue", r: 40, g: 120, b: 255 },
    { name: "Purple", r: 180, g: 60, b: 255 },
    { name: "Orange", r: 255, g: 140, b: 0 },
    { name: "Off", r: 0, g: 0, b: 0 },
  ];

  function hex(): string {
    return (
      "#" +
      [r, g, b]
        .map((v) => v.toString(16).padStart(2, "0"))
        .join("")
        .toUpperCase()
    );
  }

  function fromHex(value: string) {
    const m = /^#?([0-9a-f]{6})$/i.exec(value.trim());
    if (!m) return;
    const n = parseInt(m[1], 16);
    onChange((n >> 16) & 255, (n >> 8) & 255, n & 255);
  }
</script>

<div class="space-y-4">
  <div class="flex items-center gap-4">
    <div
      class="w-16 h-16 rounded-xl border border-gc-border shadow-inner shrink-0"
      style="background: rgb({r},{g},{b})"
    ></div>
    <div class="flex-1 space-y-2">
      <label class="flex items-center gap-2 text-xs text-gc-text-sec">
        <span class="w-8">R</span>
        <input
          type="range"
          min="0"
          max="255"
          value={r}
          class="flex-1 accent-gc-accent"
          oninput={(e) => onChange(+e.currentTarget.value, g, b)}
        />
        <span class="w-8 text-right tabular-nums text-gc-text">{r}</span>
      </label>
      <label class="flex items-center gap-2 text-xs text-gc-text-sec">
        <span class="w-8">G</span>
        <input
          type="range"
          min="0"
          max="255"
          value={g}
          class="flex-1 accent-gc-accent"
          oninput={(e) => onChange(r, +e.currentTarget.value, b)}
        />
        <span class="w-8 text-right tabular-nums text-gc-text">{g}</span>
      </label>
      <label class="flex items-center gap-2 text-xs text-gc-text-sec">
        <span class="w-8">B</span>
        <input
          type="range"
          min="0"
          max="255"
          value={b}
          class="flex-1 accent-gc-accent"
          oninput={(e) => onChange(r, g, +e.currentTarget.value)}
        />
        <span class="w-8 text-right tabular-nums text-gc-text">{b}</span>
      </label>
    </div>
  </div>

  <div class="flex items-center gap-2">
    <span class="text-xs text-gc-text-sec">Hex</span>
    <input
      type="text"
      value={hex()}
      class="flex-1 bg-gc-surface-2 border border-gc-border rounded-lg px-2 py-1 text-sm font-mono text-gc-text"
      onchange={(e) => fromHex(e.currentTarget.value)}
    />
  </div>

  <div class="flex flex-wrap gap-2">
    {#each presets as p}
      <button
        class="px-2.5 py-1 text-xs rounded-lg border border-gc-border hover:border-gc-accent transition"
        style="background: rgb({p.r},{p.g},{p.b}); color: {(p.r + p.g + p.b) > 400 ? '#111' : '#eee'}"
        onclick={() => onChange(p.r, p.g, p.b)}
        title={p.name}
      >
        {p.name}
      </button>
    {/each}
  </div>
</div>
