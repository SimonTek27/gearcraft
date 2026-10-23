<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Home from "./routes/Home.svelte";
  import Engine from "./routes/Engine.svelte";
  import Sonar from "./routes/Sonar.svelte";
  import type { Route } from "./lib/types";
  import { consumePendingRoute } from "./lib/store";

  let current: Route = $state("home");

  function navigate(route: Route) {
    current = route;
  }

  onMount(() => {
    const id = setInterval(() => {
      const next = consumePendingRoute();
      if (next) current = next;
    }, 50);
    return () => clearInterval(id);
  });
</script>

<div class="flex flex-col h-full w-full bg-gc-bg">
  <TopBar />
  <div class="flex flex-1 min-h-0">
    <Sidebar {current} onNavigate={navigate} />
    <main class="flex-1 min-w-0 bg-gc-bg">
      {#if current === "home"}
        <Home />
      {:else if current === "engine"}
        <Engine />
      {:else if current === "sonar"}
        <Sonar />
      {/if}
    </main>
  </div>
</div>
