<script lang="ts">
    import { onMount, onDestroy } from 'svelte';

    type Row = {
        ts: number;
        actor_user: string;
        action: string;
        target: string;
        ip: string;
        ua: string;
    };

    let rows: Row[] = [];
    let q = '';
    let action = '';
    let auto = true;
    let err = '';
    let loading = false;
    let timer: any;

    async function load() {
        if (loading) return;
        loading = true;
        err = '';

        try {
            const resp = await fetch('/api/admin/audit?limit=100', { credentials: 'include' });

            if (resp.status === 401 || resp.status === 403) {
                err = 'Unauthorized — please sign in again';
                auto = false;        // stop polling if we lost auth
                rows = [];
            } else if (resp.ok) {
                const data = await resp.json().catch(() => []);
                rows = Array.isArray(data) ? data as Row[] : [];
            } else {
                err = `HTTP ${resp.status}`;
                rows = [];
            }
        } catch (e) {
            err = (e as Error).message || 'Load failed';
            rows = [];
        } finally {
            loading = false;
        }
    }

    function filtered() {
        const s = q.trim().toLowerCase();
        return rows.filter(r =>
            (!action || r.action.includes(action)) &&
            (!s || `${r.actor_user} ${r.action} ${r.target} ${r.ip} ${r.ua}`
                .toLowerCase()
                .includes(s))
        );
    }

    onMount(() => {
        load();
        timer = setInterval(() => { if (auto) load(); }, 5000);
    });

    onDestroy(() => { if (timer) clearInterval(timer); });
</script>

<section class="section">
    <h2>Audit</h2>

    <div class="toolbar">
        <input class="input" placeholder="filter…" bind:value={q} />
        <select class="input narrow" bind:value={action}>
            <option value="">all actions</option>
            <option>LOGIN_SUCCESS</option>
            <option>LOGIN_FAIL_BADPW</option>
            <option>LOGOUT</option>
            <option>ADMIN_CREATE_CLIENT</option>
            <option>ADMIN_BUNDLE_CLIENT</option>
            <option>ADMIN_REVOKE_CLIENT</option>
            <option>ADMIN_SAVE_CCD</option>
        </select>
        <label class="check"><input type="checkbox" bind:checked={auto} /> auto-refresh</label>
        <button class="btn" on:click={load} disabled={loading}>Refresh</button>
    </div>

    {#if err}
        <div class="err">{err}</div>
    {/if}
    {#if !err && !loading && rows.length === 0}
        <div class="muted">No events.</div>
    {/if}

    <table class="table">
        <thead>
        <tr>
            <th>Time</th><th>Actor</th><th>Action</th><th>Target</th><th>IP</th><th>UA</th>
        </tr>
        </thead>
        <tbody>
        {#each filtered() as r}
            <tr>
                <td>{new Date(r.ts * 1000).toLocaleString()}</td>
                <td>{r.actor_user}</td>
                <td>{r.action}</td>
                <td>{r.target}</td>
                <td>{r.ip}</td>
                <td class="ua">{r.ua}</td>
            </tr>
        {/each}
        </tbody>
    </table>
</section>

