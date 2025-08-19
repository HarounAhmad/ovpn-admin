<script lang="ts">
    import { api } from '../lib/api'
    import { onMount } from 'svelte'

    type Issued = { serial?: string; cn: string; profile: string; not_after: string }

    let cn = ''
    let include_key = true
    let creating = false
    let creatingErr = ''

    let issuing = false
    let bundleCN = ''
    let bundleErr = ''

    let issued: Issued[] = []

    // NOTE: your api helper should prefix /api. If it doesn't, change to '/api/admin/issued?...'
    async function refreshIssued() {
        try {
            const list = await api.get<Issued[]>('/admin/issued?limit=50')
            issued = (list ?? []).sort((a, b) => a.cn.localeCompare(b.cn))
        } catch (e) {
            // keep quiet on dashboard; you can surface if you prefer
            console.error('issued load failed', e)
            issued = []
        }
    }

    async function createClient() {
        const newCN = cn.trim()
        if (!newCN) return
        creating = true
        creatingErr = ''
        try {
            // Backend may return cn/profile/not_after. If not, we still insert optimistically.
            const resp = await api.post<{ cn?: string; profile?: string; not_after?: string }>(
                '/admin/clients',
                { cn: newCN, include_key }
            )
            const row: Issued = {
                cn: resp?.cn ?? newCN,
                profile: resp?.profile ?? 'client',
                not_after: resp?.not_after ?? '—',
            }
            // Optimistic insert (dedupe by CN)
            issued = [row, ...issued.filter(i => i.cn !== row.cn)]
            cn = ''

            // Optional: also refresh from server to pick up serial/real not_after
            // await refreshIssued()
        } catch (e: any) {
            // If your api wrapper exposes status codes, you can special-case 409 here.
            creatingErr = e?.message ?? String(e)
        } finally {
            creating = false
        }
    }

    async function downloadBundle(name: string) {
        issuing = true
        bundleErr = ''
        bundleCN = name
        try {
            const blob = await api.postBlob(
                `/admin/clients/${encodeURIComponent(name)}/bundle`,
                { include_key: true }
            )
            const url = URL.createObjectURL(blob)
            const a = document.createElement('a')
            a.href = url
            a.download = `${name}.zip`
            document.body.appendChild(a)
            a.click()
            a.remove()
            URL.revokeObjectURL(url)
        } catch (e) {
            bundleErr = String(e)
        } finally {
            issuing = false
            bundleCN = ''
        }
    }

    onMount(refreshIssued)
</script>

<section class="pad grid">
    <h2>Clients</h2>

    <div class="card grid">
        <div class="row">
            <input class="input" placeholder="common name" bind:value={cn} />
            <label class="row"><input type="checkbox" bind:checked={include_key} /> include key</label>
            <button class="btn primary" disabled={creating || !cn.trim()} on:click|preventDefault={createClient}>
                {creating ? 'Creating…' : 'Create'}
            </button>
        </div>
        {#if creatingErr}<div class="muted">Error: {creatingErr}</div>{/if}
    </div>

    <div class="card">
        <h3>Issued</h3>
        {#if issued.length === 0}
            <div class="muted">No certificates yet.</div>
        {:else}
            <table class="table">
                <thead>
                <tr><th style="text-align:left">CN</th><th>Profile</th><th>Not After</th><th></th></tr>
                </thead>
                <tbody>
                {#each issued as it}
                    <tr>
                        <td>{it.cn}</td>
                        <td class="muted">{it.profile}</td>
                        <td class="muted">{it.not_after}</td>
                        <td style="text-align:right">
                            <button
                                    class="btn"
                                    disabled={issuing && bundleCN === it.cn}
                                    on:click={() => downloadBundle(it.cn)}
                            >
                                {issuing && bundleCN === it.cn ? 'Bundling…' : 'Bundle'}
                            </button>
                        </td>
                    </tr>
                {/each}
                </tbody>
            </table>
        {/if}
        {#if bundleErr}<div class="muted">Error: {bundleErr}</div>{/if}
    </div>
</section>
