<script lang="ts">
    import { getJson, putJson } from '../lib/api'
    import { toast } from '../lib/toast'
    import { onMount, onDestroy } from 'svelte'
    let cn = ''
    let content = ''
    let busy = false
    let err = ''

    async function load() {
        if (!cn) return
        busy = true; err = ''
        try {
            const r = await getJson<{ content: string }>(`/admin/ccd/${encodeURIComponent(cn)}`)
            content = r?.content || ''
            toast('Loaded')
        } catch (e) {
            err = (e as Error).message; content = ''
        } finally { busy = false }
    }

    async function save() {
        if (!cn) return
        busy = true; err = ''
        try { await putJson(`/admin/ccd/${encodeURIComponent(cn)}`, { content }); toast('Saved') }
        catch (e) { err = (e as Error).message }
        finally { busy = false }
    }

    function onKey(e: KeyboardEvent){
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase()==='s') { e.preventDefault(); save() }
    }
    onMount(()=>document.addEventListener('keydown', onKey))
    onDestroy(()=>document.removeEventListener('keydown', onKey))
</script>

<section class="section" style="display:grid;gap:10px">
    <h2>CCD Editor</h2>
    {#if err}<div style="color:#b00020">{err}</div>{/if}
    <div style="display:flex;gap:8px;flex-wrap:wrap">
        <input class="input" style="max-width:280px" bind:value={cn} placeholder="alice-laptop" on:keydown={(e)=>{ if(e.key==='Enter') load() }} />
        <button class="btn" on:click={load} disabled={!cn||busy}>Load</button>
        <button class="btn primary" on:click={save} disabled={!cn||busy}>Save</button>
    </div>
    <textarea class="input" bind:value={content} rows="18" style="font-family:monospace"></textarea>
    <div class="muted">Shortcut: Ctrl+S to save</div>
</section>
