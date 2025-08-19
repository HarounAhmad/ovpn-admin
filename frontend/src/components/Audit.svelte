<script lang="ts">
    import { onMount, onDestroy } from 'svelte'
    import { getJson } from '../lib/api'
    let rows: { ts:number; actor_user:string; action:string; target:string; ip:string; ua:string }[] = []
    let timer:any
    async function load() {
        const r = await getJson<typeof rows>('/admin/audit?limit=100')
        rows = r
    }
    onMount(() => { load(); timer = setInterval(load, 5000) })
    onDestroy(() => { if (timer) clearInterval(timer) })
</script>

<section style="padding:16px">
    <h2>Audit</h2>
    <table border="1" cellpadding="6" cellspacing="0">
        <thead><tr><th>Time</th><th>Actor</th><th>Action</th><th>Target</th><th>IP</th><th>UA</th></tr></thead>
        <tbody>
        {#each rows as r}
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
