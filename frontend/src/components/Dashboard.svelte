<script lang="ts">
    import { api } from '../lib/api'
    import { session } from '../lib/store'
    import { onMount } from 'svelte'
    let health = '…'
    let me: any = null
    let audit: any[] = []
    let err = ''
    onMount(async () => {
        try {
            const h = await api.get<{ok:boolean}>('/health')
            health = h.ok ? 'ok' : 'down'
            me = await api.get('/me')
            audit = await api.get('/admin/audit?limit=20').catch(()=>[])
        } catch (e) { err = String(e) }
    })
</script>

<section class="pad grid">
    <h2>Dashboard</h2>
    {#if err}<div class="muted">Error: {err}</div>{/if}
    <div class="card row">
        <div>API: <b>{health ?? '—'}</b></div>
        <div>Daemon: <b>{health ?? '—'}</b></div>
        <div>Managment: <b>{health ?? '—'}</b></div>

        {#if me}<div class="muted" style="margin-left:16px">Signed in as {me.username}</div>{/if}
    </div>

    {#if audit.length}
        <div class="card">
            <h3>Recent audit</h3>
            <ul>
                {#each audit as a}
                    <li class="muted">{a.ts} · {a.actor_user} · {a.action} · {a.target}</li>
                {/each}
            </ul>
        </div>
    {/if}
</section>
