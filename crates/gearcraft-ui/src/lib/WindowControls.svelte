<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  async function minimize() {
    try {
      await getCurrentWindow().minimize();
    } catch {}
  }
  async function toggleMaximize() {
    try {
      const win = getCurrentWindow();
      if (await win.isMaximized()) await win.unmaximize();
      else await win.maximize();
    } catch {}
  }
  async function close() {
    try {
      await getCurrentWindow().close();
    } catch {}
  }
</script>

<div class="flex items-center gap-0.5 ml-1">
  <button
    class="w-8 h-7 flex items-center justify-center rounded hover:bg-gc-hover text-gc-text-sec hover:text-gc-text transition"
    title="Minimize"
    onclick={minimize}
  >
    <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
      <rect y="5" width="12" height="1.5" rx="0.5" />
    </svg>
  </button>
  <button
    class="w-8 h-7 flex items-center justify-center rounded hover:bg-gc-hover text-gc-text-sec hover:text-gc-text transition"
    title="Maximize"
    onclick={toggleMaximize}
  >
    <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.4">
      <rect x="1" y="1" width="10" height="10" rx="1" />
    </svg>
  </button>
  <button
    class="w-8 h-7 flex items-center justify-center rounded hover:bg-red-500/80 text-gc-text-sec hover:text-white transition"
    title="Close"
    onclick={close}
  >
    <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.6">
      <path d="M2 2l8 8M10 2L2 10" />
    </svg>
  </button>
</div>
