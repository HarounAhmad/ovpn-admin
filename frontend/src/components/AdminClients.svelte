<script lang="ts">
    import { api } from '../lib/api'
    import { session } from '../lib/store'
    import { onMount } from 'svelte'
    let cn = ''
    let include_key = true
    let creating = false
    let creatingErr = ''
    let issuing = false
    let bundleCN = ''
    let bundleErr = ''
    let issued: Array<{serial:string; cn:string; profile:string; not_after:string}> = []

    async function refreshIssued() {
        issued = await api.get('/admin/issued?limit=50')
    }

    async function createClient() {
        creating = true; creatingErr = ''
        try {
            await api.post('/admin/clients', { cn, include_key })
            cn = ''
            await refreshIssued()
        } catch (e) {
            creatingErr = String(e)
        } finally { creating = false }
    }

    async function downloadBundle(cn: string) {
        issuing = true; bundleErr = ''
        try {
            const b = await api.postBlob(`/admin/clients/${encodeURIComponent(cn)}/bundle`, { include_key: true })
            const url = URL.createObjectURL(b)
            const a = document.createElement('a')
            a.href = url
            a.download = `${cn}.zip`
            document.body.appendChild(a)
            a.click()
            a.remove()
            URL.revokeObjectURL(url)
        } catch (e) {
            bundleErr = String(e)
        } finally { issuing = false }
    }

    onMount(refreshIssued)
</script>

<section class="pad grid">
    <h2>Clients</h2>
    <div class="card grid">
        <div class="row">
            <input class="input" placeholder="common name" bind:value={cn} />
            <label class="row"><input type="checkbox" bind:checked={include_key} /> include key</label>
            <button class="btn primary" disabled={creating || !cn} on:click|preventDefault={createClient}>
                {creating ? 'Creating…' : 'Create'}
            </button>
        </div>
        {#if creatingErr}<div class="muted">Error: {creatingErr}</div>{/if}
    </div>

    <div class="card">
        <h3>Issued</h3>
        <table style="width:100%;border-collapse:collapse">
            <thead><tr><th style="text-align:left">CN</th><th>Profile</th><th>Not After</th><th></th></tr></thead>
            <tbody>
            {#each issued as it}
                <tr>
                    <td>{it.cn}</td>
                    <td class="muted">{it.profile}</td>
                    <td class="muted">{it.not_after}</td>
                    <td style="text-align:right">
                        <button class="btn" disabled={issuing} on:click={() => downloadBundle(it.cn)}>
                            {issuing && bundleCN===it.cn ? '…' : 'Bundle'}
                        </button>
                    </td>
                </tr>
            {/each}
            </tbody>
        </table>
        {#if bundleErr}<div class="muted">Error: {bundleErr}</div>{/if}
    </div>
</section>
