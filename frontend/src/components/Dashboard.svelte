<script lang="ts">
    import { onMount } from 'svelte';


    const fetchStatus = async (path: string): Promise<string> => {
        const r = await fetch(`/api${path}`, { credentials: 'include' });
        if (!r.ok) return 'down';

        const ct = r.headers.get('content-type') ?? '';
        if (ct.includes('application/json')) {
            const j: any = await r.json().catch(() => ({}));
            if (typeof j?.ok === 'boolean') return j.ok ? 'ok' : 'down';
            if (typeof j?.status === 'string') return j.status.toLowerCase();
            return 'ok';
        } else {
            const t = (await r.text()).trim().toLowerCase();
            if (t === 'ok' || t === 'pong' || t === 'healthy') return 'ok';
            if (!t) return 'down';
            return t;
        }
    };

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
            apiStatus = await fetchStatus('/health');
        } catch (e) {
            apiStatus = 'down';
            err ||= String(e);
        }

        daemonStatus = apiStatus;
        mgmtStatus = apiStatus;

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
