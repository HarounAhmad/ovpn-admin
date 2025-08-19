<script lang="ts">
    import { getJson, putJson } from '../lib/api'
    import { toast } from '../lib/toast'
    let cn = ''
    let content = ''
    let busy = false

    async function load() {
        if (!cn) return
        busy = true
        try {
            const r = await getJson<{ content: string }>(`/admin/ccd/${encodeURIComponent(cn)}`)
            content = r?.content || ''
            toast('Loaded')
        } catch (e) {
            content = ''
            toast(`Load failed: ${(e as Error).message}`)
        } finally { busy = false }
    }

    async function save() {
        if (!cn) return
        busy = true
        try {
            await putJson(`/admin/ccd/${encodeURIComponent(cn)}`, { content })
            toast('Saved')
        } catch (e) {
            toast(`Save failed: ${(e as Error).message}`)
        } finally { busy = false }
    }
</script>

<section style="padding:16px;display:grid;gap:10px">
    <h2>CCD Editor</h2>
    <label>Common Name <input class="input" bind:value={cn} placeholder="alice-laptop" /></label>
    <div style="display:flex;gap:8px">
        <button class="btn" on:click={load} disabled={!cn || busy}>Load</button>
        <button class="btn" on:click={save} disabled={!cn || busy}>Save</button>
    </div>
    <textarea class="input" bind:value={content} rows="16" style="width:100%;font-family:monospace"></textarea>
</section>
