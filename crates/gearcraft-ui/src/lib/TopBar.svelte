<script lang="ts">
  import WindowControls from "./WindowControls.svelte";

  interface Props {
    title?: string;
  }
  let { title = "GearCraft" }: Props = $props();
</script>

<header
  class="h-10 flex items-center justify-between px-3 bg-gc-titlebar border-b border-gc-border select-none shrink-0"
  data-tauri-drag-region
>
  <div class="flex items-center gap-2" data-tauri-drag-region>
    <div
      class="w-3 h-3 rounded-full bg-gc-accent shadow-[0_0_8px_rgba(0,231,160,0.5)]"
    ></div>
    <span class="text-sm font-semibold tracking-wide text-gc-text">{title}</span>
  </div>

  <div class="flex items-center gap-1">
    <button
      class="p-1.5 rounded hover:bg-gc-hover text-gc-text-sec hover:text-gc-text transition"
      title="Settings"
      onclick={() => {}}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <circle cx="12" cy="12" r="3" />
        <path
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
        />
      </svg>
    </button>
    <WindowControls />
  </div>
</header>
