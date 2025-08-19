<script lang="ts">
    import { api } from '../lib/api'
    import { route } from '../lib/hashRouter'
    import { onMount } from 'svelte'
    let cn = ''
    let content = ''
    let status = ''
    $: { const m = $route.match(/^\/ccd\/(.+)$/); cn = m ? decodeURIComponent(m[1]) : '' }

    async function load() {
        status = 'loading'
        try { content = await api.get(`/admin/ccd/${encodeURIComponent(cn)}`); status = '' }
        catch (e) { status = String(e) }
    }
    async function save() {
        status = 'saving'
        try { await api.put(`/admin/ccd/${encodeURIComponent(cn)}`, { content }); status = 'saved' }
        catch (e) { status = String(e) }
    }
    onMount(load)
</script>

<section class="pad grid">
    <h2>CCD: {cn}</h2>
    {#if status}<div class="muted">{status}</div>{/if}
    <textarea class="input" rows="16" bind:value={content}></textarea>
    <div class="row">
        <button class="btn primary" on:click|preventDefault={save}>Save</button>
        <button class="btn" on:click|preventDefault={load}>Reload</button>
    </div>
</section>
