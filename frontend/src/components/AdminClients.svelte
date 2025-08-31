<script lang="ts">
    import { onMount } from 'svelte'
    import { api } from '../lib/api'
    import { goto } from '../lib/hashRouter'

    type Issued = {
        serial?: string
        cn: string
        profile: string
        not_after: string
        revoked?: boolean
        revoked_at?: string | null
    }

    type CreateResp = {
        cn: string
        profile?: string
        not_after?: string
        passphrase: string
        serial?: string
        cdc?: string
    }

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
    let ccd;

    let last: { cn: string; passphrase: string; serial?: string; not_after?: string } = {
        cn: '', passphrase: '', serial: '', not_after: ''
    }
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
            const resp = await api.post<CreateResp>('/admin/clients', {
                cn: newCN,
                include_key,
                passphrase: passphrase.trim() || undefined,
                ccd: ccd?.trim() || undefined
            })

            last = {
                cn: resp?.cn ?? newCN,
                passphrase: resp?.passphrase ?? '',
                serial: resp?.serial ?? '',
                not_after: resp?.not_after ?? '',
                ccd: resp?.ccd ?? ''
            }
            isNew = true

            const row: Issued = {
                cn: resp?.cn ?? newCN,
                profile: resp?.profile ?? 'client',
                not_after: resp?.not_after ?? '—',
                revoked: false
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

    async function revokeClient(name: string) {
        if (!confirm(`Revoke certificate for "${name}"? This cannot be undone.`)) return
        revoking = true
        revokeErr = ''
        revokeCN = name
        try {
            await api.post(`/admin/clients/${encodeURIComponent(name)}/revoke`, {})
            await refreshIssued()
        } catch (e) {
            revokeErr = String(e)
        } finally {
            revoking = false
            revokeCN = ''
        }
    }

    function openCcd(name: string) {
        goto(`/ccd/${encodeURIComponent(name)}`)
    }

    onMount(refreshIssued)
</script>

<section class="pad grid">
    <h2>Clients</h2>

    <div class="card grid">
        <div class="row wrap">
            <input class="input" placeholder="common name" bind:value={cn} />
            <input class="input" placeholder="passphrase (optional)" bind:value={passphrase} />
            <label class="row"><input type="checkbox" bind:checked={include_key} /> include key</label>
            <button class="btn primary" disabled={creating || !cn.trim()} on:click|preventDefault={createClient}>
                {creating ? 'Creating…' : 'Create'}
            </button>
        </div>
        <div class="row wrap">
            <input class="input" placeholder="ccd" bind:value={ccd} />
        </div>
        {#if creatingErr}<div class="msg err">Error: {creatingErr}</div>{/if}
    </div>

    {#if isNew}
        <div class="card">
            <h3>New client</h3>
            <div class="last-grid">
                <div><span class="muted">CN</span><div class="kv">{last.cn}</div></div>
                <div><span class="muted">Passphrase</span><div class="kv mono">{last.passphrase || '—'}</div></div>
                <div><span class="muted">Serial</span><div class="kv mono">{last.serial || '—'}</div></div>
                <div><span class="muted">Not After</span><div class="kv">{last.not_after || '—'}</div></div>
                <div><span class="muted">ccd</span><div class="kv">{last.ccd || '—'}</div></div>
            </div>
            <div class="actions">
                <button class="btn" on:click={() => downloadBundle(last.cn)} disabled={!last.cn}>Bundle</button>
                <button class="btn" on:click={() => openCcd(last.cn)} disabled={!last.cn}>Edit CCD</button>
            </div>
        </div>
    {/if}

    <div class="card">
        <div class="row between">
            <h3>Issued</h3>
            <div class="row gap">
                <button class="btn" on:click={refreshIssued}>Refresh</button>
            </div>
        </div>

        {#if issued.length === 0}
            <div class="muted">No certificates yet.</div>
        {:else}
            <table class="table">
                <thead>
                <tr>
                    <th class="left">CN</th>
                    <th>Profile</th>
                    <th>Not After</th>
                    <th>Status</th>
                    <th class="right"></th>
                </tr>
                </thead>
                <tbody>
                {#each issued as it}
                    <tr>
                        <td class="left">{it.cn}</td>
                        <td class="muted">{it.profile}</td>
                        <td class="muted">{it.not_after}</td>
                        <td>
                            {#if it.revoked}
                                <span class="badge err">revoked</span>
                                {#if it.revoked_at}
                                    <span class="muted small">{it.revoked_at}</span>
                                {/if}
                            {:else}
                                <span class="badge ok">active</span>
                            {/if}
                        </td>
                        <td class="right actions">
                            <button
                                    class="btn"
                                    disabled={(issuing && bundleCN === it.cn) || it.revoked}
                                    on:click={() => downloadBundle(it.cn)}
                            >
                                {issuing && bundleCN === it.cn ? 'Bundling…' : 'Bundle'}
                            </button>

                            <button
                                    class="btn"
                                    on:click={() => openCcd(it.cn)}
                            >
                                Edit CCD
                            </button>

                            <button
                                    class="btn danger"
                                    disabled={issuing || revoking || it.revoked}
                                    on:click={() => revokeClient(it.cn)}
                                    aria-busy={revoking && revokeCN===it.cn}
                            >
                                {revoking && revokeCN===it.cn ? 'Revoking…' : 'Revoke'}
                            </button>
                        </td>
                    </tr>
                {/each}
                </tbody>
            </table>
        {/if}

        {#if bundleErr}<div class="msg err">Bundle error: {bundleErr}</div>{/if}
        {#if revokeErr}<div class="msg err">Revoke error: {revokeErr}</div>{/if}
    </div>
</section>
