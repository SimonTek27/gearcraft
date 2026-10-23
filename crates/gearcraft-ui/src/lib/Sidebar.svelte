<script lang="ts">
  import type { Route } from "./types";

  interface Props {
    current: Route;
    onNavigate: (route: Route) => void;
  }
  let { current, onNavigate }: Props = $props();

  const items: { id: Route; label: string; icon: string }[] = [
    { id: "home", label: "Home", icon: "home" },
    { id: "engine", label: "Engine", icon: "engine" },
    { id: "sonar", label: "Sonar", icon: "sonar" },
  ];
</script>

<aside
  class="w-[150px] bg-gc-sidebar border-r border-gc-border flex flex-col py-4 shrink-0"
>
  <nav class="flex flex-col gap-1 px-2">
    {#each items as item}
      <button
        class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors
          {current === item.id
          ? 'bg-gc-accent/10 text-gc-accent'
          : 'text-gc-text-sec hover:bg-gc-hover hover:text-gc-text'}"
        onclick={() => onNavigate(item.id)}
      >
        {#if item.icon === "home"}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
            <polyline points="9 22 9 12 15 12 15 22" />
          </svg>
        {:else if item.icon === "engine"}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        {:else}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M3 18v-6a9 9 0 0 1 18 0v6" />
            <path d="M21 19a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3zM3 19a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-2-2H3z" />
          </svg>
        {/if}
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="mt-auto px-3 pb-2">
    <div class="text-[10px] text-gc-text-sec/70 uppercase tracking-wider">v0.1.0</div>
  </div>
</aside>
