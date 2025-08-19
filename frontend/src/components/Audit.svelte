<script lang="ts">
    import { onMount, onDestroy } from 'svelte'
    import { getJson } from '../lib/api'

    type Row = { ts:number; actor_user:string; action:string; target:string; ip:string; ua:string }
    let rows: Row[] = []
    let q = ''
    let action = ''
    let auto = true
    let err = ''
    let loading = false
    let timer:any

    async function load(){
        loading = true
        err = ''
        try {
            const r = await getJson<Row[]>('/admin/audit?limit=200') // becomes /api/admin/audit
            rows = Array.isArray(r) ? r : []
        } catch (e) {
            err = (e as Error).message || 'load failed'
            // stop the timer on auth loss to avoid flicker/spam
            if (err.toLowerCase().includes('unauthorized')) auto = false
            rows = []
        } finally {
            loading = false
        }
    }

    function filtered(){
        const s = q.trim().toLowerCase()
        return rows.filter(r =>
            (!action || r.action.includes(action)) &&
            (!s || `${r.actor_user} ${r.action} ${r.target} ${r.ip} ${r.ua}`.toLowerCase().includes(s))
        )
    }

    onMount(()=>{ load(); timer = setInterval(()=>{ if (auto) load() }, 5000) })
    onDestroy(()=>{ if (timer) clearInterval(timer) })
</script>

<section class="section">
    <h2>Audit</h2>

    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:10px">
        <input class="input" placeholder="filter…" bind:value={q} />
        <select class="input" style="max-width:200px" bind:value={action}>
            <option value="">all actions</option>
            <option>LOGIN_SUCCESS</option>
            <option>LOGIN_FAIL_BADPW</option>
            <option>LOGOUT</option>
            <option>ADMIN_CREATE_CLIENT</option>
            <option>ADMIN_BUNDLE_CLIENT</option>
            <option>ADMIN_REVOKE_CLIENT</option>
            <option>ADMIN_SAVE_CCD</option>
        </select>
        <label><input type="checkbox" bind:checked={auto} /> auto-refresh</label>
        <button class="btn" on:click={load} disabled={loading}>Refresh</button>
    </div>

    {#if err}<div style="color:#b00020;margin-bottom:8px">{err}</div>{/if}
    {#if !err && !loading && rows.length === 0}<div class="muted">No events.</div>{/if}

    <table class="table">
        <thead><tr><th>Time</th><th>Actor</th><th>Action</th><th>Target</th><th>IP</th><th>UA</th></tr></thead>
        <tbody>
        {#each filtered() as r}
            <tr>
                <td>{new Date(r.ts*1000).toLocaleString()}</td>
                <td>{r.actor_user}</td>
                <td>{r.action}</td>
                <td>{r.target}</td>
                <td>{r.ip}</td>
                <td>{r.ua}</td>
            </tr>
        {/each}
        </tbody>
    </table>
</section>
