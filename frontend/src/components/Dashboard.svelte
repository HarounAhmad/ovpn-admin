<script lang="ts">
    import { onMount } from 'svelte';
    import {api} from "../lib/api";


    type Status = 'ok' | 'down';
    type HealthPayload = {
        api?: { ok: boolean };
        daemon?: { ok: boolean };
        agent?: { ok: boolean };
    };

    export async function fetchStatus(): Promise<{ api: Status; daemon: Status; agent: Status }> {
        const r = await fetch('/api/health', { credentials: 'include' });
        if (!r.ok) return { api: 'NOK', daemon: 'NOK', agent: 'NOK' };

        const j: HealthPayload = await r.json().catch(() => ({}));
        const toStatus = (v: any): Status =>
            v && typeof v.ok === 'boolean' ? (v.ok ? 'ok' : 'NOK') : 'NOK';

        return {
            api: toStatus(j.api),
            daemon: toStatus(j.daemon),
            agent: toStatus(j.agent),
        };
    }

    const cls = (s: string | null) => {
        const v = (s ?? '—').toLowerCase();
        if (v === 'ok' || v === 'pong' || v === 'healthy') return 'ok';
        if (v === 'degraded') return 'warn';
        if (v === '—' || v === '') return 'muted';
        return 'err';
    };

    const fmtTs = (ts: number) =>
        new Date(ts * 1000).toLocaleString(undefined, {
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit'
        });

    let apiStatus: string | null = null;
    let daemonStatus: string | null = null;
    let mgmtStatus: string | null = null;

    let me: { username: string; roles?: string[] } | null = null;
    let audit: Array<{ ts: number; actor_user: string; action: string; target: string }> = [];
    let err = '';

    onMount(async () => {
        try {
            const h = await fetchStatus();
            console.log(h)
            apiStatus = h.api;
            daemonStatus = h.daemon;
            mgmtStatus = h.agent;
        } catch (e) {
            apiStatus = 'NOK';
            err ||= String(e);
        }


        try {
            const r = await fetch('/api/me', { credentials: 'include' });
            me = r.ok ? await r.json() : null;
        } catch { me = null; }

        try {
            const r = await fetch('/api/admin/audit?limit=20', { credentials: 'include' });
            audit = r.ok ? await r.json() : [];
        } catch { audit = []; }
    });
</script>

<section class="pad grid">
    <h2>Dashboard</h2>
    {#if err}<div class="muted">Error: {err}</div>{/if}

    <div class="card status">
        <div class="row"><div class="label">API</div>        <div class="value"><span class="badge {cls(apiStatus)}">{apiStatus ?? '—'}</span></div></div>
        <div class="row"><div class="label">Daemon</div>     <div class="value"><span class="badge {cls(daemonStatus)}">{daemonStatus ?? '—'}</span></div></div>
        <div class="row"><div class="label">Management</div> <div class="value"><span class="badge {cls(mgmtStatus)}">{mgmtStatus ?? '—'}</span></div></div>
        {#if me}<div class="meta">Signed in as {me.username}</div>{/if}
    </div>

    {#if audit.length}
        <div class="card">
            <div class="card-head">
                <h3>Recent audit</h3>
                <div class="count">{audit.length}</div>
            </div>
            <ul class="audit">
                {#each audit as a}
                    <li>
                        <span class="ts">{fmtTs(a.ts)}</span>
                        <span class="actor">{a.actor_user}</span>
                        <span class="action">{a.action}</span>
                        <span class="target">{a.target}</span>
                    </li>
                {/each}
            </ul>
        </div>
    {/if}
</section>
