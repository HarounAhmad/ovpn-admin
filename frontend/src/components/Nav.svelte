<script lang="ts">
    import { goto } from '../lib/hashRouter'
    import { logout, session } from '../lib/store'
</script>

<nav class="nav">
    <a href="#/">OVPN Admin</a>
    <a href="#/clients">Clients</a>
    <a href="#/ccd">CCD</a>
    <a href="#/audit">Audit</a>
    <div style="flex:1"></div>
    {#if $session}<span class="muted">{$session.username}</span>{/if}
    <button class="btn" on:click={logout}>Logout</button>
</nav>
