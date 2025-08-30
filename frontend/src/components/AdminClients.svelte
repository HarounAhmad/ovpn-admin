<script lang="ts">
    import { api } from '../lib/api'
    import { onMount } from 'svelte'

    type Issued = { serial?: string; cn: string; profile: string; not_after: string, revoked?: boolean, revoked_at?: string}

    let cn = ''
    let passphrase = ''
    let include_key = true
    let creating = false
    let creatingErr = ''

    let issuing = false
    let bundleCN = ''
    let bundleErr = ''

    let revoking = false
    let revokeCN = ''
    let revokeErr = ''

    let issued: Issued[] = []

    let last = { cn: '', passphrase: '', serial: '', not_after: ''}
    let isNew = false

    async function refreshIssued() {
        try {
            const list = await api.get<Issued[]>('/admin/issued?limit=50')
            issued = (list ?? []).sort((a, b) => a.cn.localeCompare(b.cn))
        } catch (e) {
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
            const resp = await api.post<{ cn?: string; profile?: string; not_after?: string; passphrase: string }>(
                '/admin/clients',
                { cn: newCN, include_key, passphrase: passphrase.trim() || undefined}
            )
            last = resp
            isNew = true
            const row: Issued = {
                cn: resp?.cn ?? newCN,
                profile: resp?.profile ?? 'client',
                not_after: resp?.not_after ?? '—',
            }
            issued = [row, ...issued.filter(i => i.cn !== row.cn)]
            cn = ''
            passphrase = ''

        } catch (e: any) {
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

    async function revokeClient(cn: string) {
        if (!confirm(`Revoke certificate for "${cn}"? This cannot be undone.`)) return
        revoking = true; revokeErr = ''; revokeCN = cn
        try {
            await api.post(`/admin/clients/${encodeURIComponent(cn)}/revoke`, {})
            await refreshIssued()
        } catch (e) {
            revokeErr = String(e)
        } finally { revoking = false; revokeCN = '' }
    }

    onMount(refreshIssued)
</script>

<section class="pad grid">
    <h2>Clients</h2>

    <div class="card grid">
        <div class="row">
            <input class="input" placeholder="common name" bind:value={cn} />
            <input class="input" placeholder="passphrase" bind:value={passphrase} />
            <label class="row"><input type="checkbox" bind:checked={include_key} /> include key</label>
            <button class="btn primary" disabled={creating || !cn.trim()} on:click|preventDefault={createClient}>
                {creating ? 'Creating…' : 'Create'}
            </button>
        </div>
        {#if creatingErr}<div class="muted">Error: {creatingErr}</div>{/if}
    </div>
    {#if isNew}
    <div class="card grid">
        <h3>New Client</h3>
        <div class="row">
            <label>CN: {last.cn}</label>
            <label>Passphrase: {last.passphrase}</label>
            <label>Serial: {last.serial}</label>
            <label>Not After: {last.not_after}</label>
        </div>
        <button
                class="btn"
                disabled={!isNew}
                on:click={() => downloadBundle(last.cn || '')}
        > Bundle </button>
    </div>
    {/if}

    <div class="card">
        <h3>Issued</h3>
        {#if issued.length === 0}
            <div class="muted">No certificates yet.</div>
        {:else}
            <table class="table">
                <thead>
                <tr><th style="text-align:left">CN</th><th>Profile</th><th>Not After</th><th>Revoked</th><th></th></tr>
                </thead>
                <tbody>
                {#each issued as it}
                    <tr>
                        <td>{it.cn}</td>
                        <td class="muted">{it.profile}</td>
                        <td class="muted">{it.not_after}</td>
                        <td class="muted">
                            {#if it.revoked}
                            <span class="badge err">revoked</span>
                            {#if it.revoked_at}
                                <span class="muted" style="margin-left:6px">{it.revoked_at}</span>
                            {/if}
                        {:else}
                            <span class="badge ok">active</span>
                        {/if}
                        </td>
                        <td style="text-align:right">
                            <button
                                    class="btn"
                                    disabled={issuing && bundleCN === it.cn}
                                    on:click={() => downloadBundle(it.cn)}
                            >
                                {issuing && bundleCN === it.cn ? 'Bundling…' : 'Bundle'}
                            </button>
                            <button
                                    class="btn danger"
                                    disabled={issuing || revoking || it.revoked}
                                    on:click={() => revokeClient(it.cn)}
                                    aria-busy={revoking && revokeCN===it.cn}
                                    >Revoke</button>
                        </td>
                    </tr>
                {/each}
                </tbody>
            </table>
        {/if}
        {#if bundleErr}<div class="muted">Error: {bundleErr}</div>{/if}
    </div>
</section>
