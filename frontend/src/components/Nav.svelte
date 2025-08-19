<script lang="ts">
    import { session } from '../lib/store'
    import { goto } from '../lib/hashRouter'
    import { logout } from '../lib/api'
    const nav = (p:string)=>()=>goto(p)
    async function doLogout(){ await logout(); location.reload() }
    $: s = $session
</script>

<nav class="navbar">
    <strong>OVPN Admin</strong>
    <button class="btn" on:click={nav('/')}>Dashboard</button>
    {#if s?.roles?.includes('ADMIN')}
        <button class="btn" on:click={nav('/clients')}>Clients</button>
        <button class="btn" on:click={nav('/ccd')}>CCD</button>
        <button class="btn" on:click={nav('/audit')}>Audit</button>
    {/if}
    <div class="spacer"></div>
    {#if s}
        <span class="muted">{$session?.username}</span>
        <button class="btn ghost" on:click={doLogout}>Logout</button>
    {/if}
</nav>
