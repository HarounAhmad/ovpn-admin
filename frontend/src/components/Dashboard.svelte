<script lang="ts">
    import { onMount } from 'svelte'
    import { getJson } from '../lib/api'
    type Health = string
    type Me = { username:string; roles:string[] }
    let health: Health | null = null
    let me: Me | null = null
    let err = ''

    async function load() {
        err = ''
        try {
            const h = await getJson<{ok:boolean}>('/health')
            health = h.ok ? 'ok' : 'down'
            me = await getJson<Me>('/me')
        } catch (e) {
            err = (e as Error).message
        }
    }
    onMount(load)
</script>

<section class="section" style="display:grid;gap:12px">
    <h2>Dashboard</h2>
    {#if err}<div style="color:#b00020">{err}</div>{/if}
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px">
        <div style="border:1px solid var(--border);border-radius:8px;padding:12px">
            <div class="muted">Health</div>
            <div style="font-weight:600;margin-top:6px"><b>API: &nbsp;</b>{health ?? '—'}</div>
            <div style="font-weight:600;margin-top:6px"><b>Daemon: &nbsp;</b>{health ?? '—'}</div>
            <div style="font-weight:600;margin-top:6px"><b>Managment: &nbsp;</b>{health ?? '—'}</div>
        </div>
        <div style="border:1px solid var(--border);border-radius:8px;padding:12px">
            <div class="muted">Session</div>
            {#if me}
                <div style="margin-top:6px">
                    <div><strong>{me.username}</strong></div>
                    <div class="muted" style="margin-top:4px">{me.roles.join(', ')}</div>
                </div>
            {:else}
                <div style="margin-top:6px">Not signed in</div>
            {/if}
        </div>
    </div>
</section>
