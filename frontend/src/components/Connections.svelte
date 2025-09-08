<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '../lib/api';

    type Row = {
        cn: string;
        real_ip: string;
        vpn_ip: string;
        bytes_in: number;
        bytes_out: number;
        connected_since: string;
        client_id?: number;
    };

    let rows: Row[] = [];
    let err = '';
    let loading = false;
    let auto = true;
    let timer: any;

    async function load() {
        loading = true; err = '';
        try {
            rows = await api.get<Row[]>('/admin/mgmt/clients');
        } catch (e:any) {
            err = e?.message ?? String(e);
            rows = [];
        } finally { loading = false; }
    }

    async function kick(r: Row) {
        try {
            await api.post('/admin/mgmt/clients/kick', r.client_id != null
                ? { client_id: r.client_id }
                : { cn: r.cn });
            // quick refresh after kick
            setTimeout(load, 500);
        } catch (e:any) {
            alert('Kick failed: ' + (e?.message ?? e));
        }
    }

    onMount(() => {
        load();
        timer = setInterval(() => { if (auto) load(); }, 5000);
    });
    $: if (!auto && timer) { clearInterval(timer); timer = null; }
</script>

<section class="card">
    <h3>Connected clients</h3>
    {#if err}<div class="chip danger">Error: {err}</div>{/if}
    <div class="row" style="margin-bottom:8px">
        <label><input type="checkbox" bind:checked={auto} /> auto-refresh</label>
        <button class="btn" on:click={load} disabled={loading}>{loading ? 'Loading…' : 'Refresh'}</button>
    </div>

    <table class="table">
        <thead>
        <tr>
            <th style="text-align:left">CN</th><th>Real IP</th><th>VPN IP</th>
            <th>In</th><th>Out</th><th>Since</th><th></th>
        </tr>
        </thead>
        <tbody>
        {#each rows as r}
            <tr>
                <td>{r.cn}</td>
                <td class="muted">{r.real_ip}</td>
                <td class="muted">{r.vpn_ip}</td>
                <td>{r.bytes_in.toLocaleString()}</td>
                <td>{r.bytes_out.toLocaleString()}</td>
                <td class="muted">{r.connected_since}</td>
                <td style="text-align:right">
                    <button class="btn danger" on:click={() => kick(r)}>Kick</button>
                </td>
            </tr>
        {/each}
        {#if rows.length === 0}
            <tr><td colspan="7" class="muted">No clients connected.</td></tr>
        {/if}
        </tbody>
    </table>
</section>
