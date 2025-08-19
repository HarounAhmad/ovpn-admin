<script lang="ts">
  import { onMount } from 'svelte'
  import { session, loading, refreshSession, hasRole } from './lib/store'
  import Nav from './components/Nav.svelte'
  import Login from './components/Login.svelte'
  import Dashboard from './components/Dashboard.svelte'
  import AdminClients from './components/AdminClients.svelte'
  import CcdEditor from './components/CcdEditor.svelte'
  import { route, goto } from './lib/hashRouter'
  import Audit from "./components/Audit.svelte";

  onMount(() => { refreshSession() })
  $: r = $route
  $: s = $session
  function guardAdmin() { if (!hasRole($session, 'ADMIN')) goto('/') }
</script>

{#if $loading}
  <div class="pad">Loading…</div>
{:else}
  {#if !$session}
    <Login />
  {:else}
    <Nav />
    {#if r === '/'}         <Dashboard />
    {:else if r.startsWith('/clients')}{guardAdmin()}<AdminClients />
    {:else if r.startsWith('/ccd')}{guardAdmin()}<CcdEditor />
    {:else if r.startsWith('/audit')}{guardAdmin()}<Audit />
    {:else}<section class="pad"><h2>Not found</h2></section>{/if}
  {/if}
{/if}
