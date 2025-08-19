<script lang="ts">
  import Nav from './components/Nav.svelte'
  import Login from './components/Login.svelte'
  import Dashboard from './components/Dashboard.svelte'
  import AdminClients from './components/AdminClients.svelte'
  import CcdEditor from './components/CcdEditor.svelte'
  import Audit from './components/Audit.svelte'
  import Toaster from './components/Toaster.svelte'
  import { route, goto } from './lib/hashRouter'
  import { session, loading, refreshSession, hasRole } from './lib/store'
  import { onMount } from 'svelte'

  onMount(() => { refreshSession() })   // call once, here only

  $: r = $route;

  // redirect non-admins away from admin routes once session is known
  $: if (!$loading && $session && !hasRole($session, 'ADMIN')) {
    if (r.startsWith('/clients') || r.startsWith('/ccd') || r.startsWith('/audit')) goto('/');
  }
</script>

{#if $loading}
  <div style="padding:16px">Loading…</div>
{:else}
  {#if !$session}
    <Login />
  {:else}
    <Nav />
    {#if r === '/'}
      <Dashboard />
    {:else if r.startsWith('/clients')}
      <AdminClients />
    {:else if r.startsWith('/ccd')}
      <CcdEditor />
    {:else if r.startsWith('/audit')}
      <Audit />
    {:else}
      <section style="padding:16px"><h2>Not found</h2></section>
    {/if}
  {/if}
{/if}
<Toaster />
