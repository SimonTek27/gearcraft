<script lang="ts">
  import type { DeviceInfo } from "../lib/types";

  interface Props {
    device: DeviceInfo;
    selected?: boolean;
    onSelect?: (id: string) => void;
  }
  let { device, selected = false, onSelect }: Props = $props();

  const kindIcon = {
    mouse: "🖱️",
    keyboard: "⌨️",
    headset: "🎧",
    other: "🔌",
  } as const;
</script>

<button
  class="gc-card w-full text-left transition hover:border-gc-accent/50
    {selected ? 'ring-1 ring-gc-accent border-gc-accent/40' : ''}"
  onclick={() => onSelect?.(device.id)}
>
  <div class="flex items-start gap-3">
    <div
      class="w-10 h-10 rounded-lg bg-gc-surface-2 flex items-center justify-center text-lg shrink-0"
    >
      {kindIcon[device.kind] ?? kindIcon.other}
    </div>
    <div class="min-w-0 flex-1">
      <div class="font-medium text-gc-text truncate">{device.name}</div>
      <div class="text-xs text-gc-muted mt-0.5">
        {device.vendor} · {device.kind}
      </div>
      <div class="text-[10px] text-gc-muted/70 mt-1 font-mono">
        {device.vendor_id.toString(16).padStart(4, "0")}:{device.product_id
          .toString(16)
          .padStart(4, "0")}
      </div>
    </div>
  </div>
</button>
