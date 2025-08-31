<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '../lib/api';
    import { route, goto } from '../lib/hashRouter';
    import { hasRole, session } from '../lib/store';

    $: path = $route;
    $: cn = path.startsWith('/ccd/') ? decodeURIComponent(path.slice('/ccd/'.length)) : '';

    type CcdItem = { cn:string; size:number; modified:number };
    let list: CcdItem[] = [];
    let filter = '';

    let content = '';
    let loading = false;
    let saving = false;
    let msg = '';
    let err = '';

    async function loadList() {
        try {
            const res = await api.get<CcdItem[] | { items: CcdItem[] }>('/admin/ccd')
            list = Array.isArray(res) ? res as CcdItem[] : (res as any).items ?? []
            console.debug('ccd list:', list)
        } catch (e) {
            err = String(e)
            list = []
        }
    }

    $: filteredList = (list ?? []).filter(x => {
        const s = (filter ?? '').trim().toLowerCase()
        return !s || x.cn.toLowerCase().includes(s)
    })

    async function load() {
        msg = ''; err = '';
        if (!cn) return;
        loading = true;
        try {
            const r = await fetch(`/api/admin/ccd/${encodeURIComponent(cn)}`, { credentials: 'include' });
            if (r.status === 404) {
                content = '';

                msg = 'No CCD exists yet for this client.';
            } else if (!r.ok) {
                throw new Error(`${r.status} ${r.statusText}`);
            } else {
                let full = JSON.parse(await r.text())
                console.log(full)
                content = full.content;
            }
        } catch (e) {
            err = (e as Error).message ?? 'load failed';
            content = '';
        } finally {
            loading = false;
        }
    }

    async function save() {
        msg = ''; err = '';
        if (!cn) { err = 'Pick a CCD from the list or enter a CN.'; return; }
        saving = true;
        try {
            await api.put(`/admin/ccd/${encodeURIComponent(cn)}`, { content });
            msg = 'Saved.';
            await loadList();
        } catch (e) {
            err = String(e);
        } finally {
            saving = false;
        }
    }

    function openCn(x: CcdItem) {
        goto(`/ccd/${encodeURIComponent(x.cn)}`);
    }

    $: if (cn) load();

    onMount(async () => {
        if (!hasRole($session, 'ADMIN')) goto('/');
        await loadList();
        if (cn) await load();
    });
</script>

<section class="pad grid ccd">
    <h2>CCD</h2>

    <div class="ccd-grid">
        <aside class="card">
            <div class="row">
                <input class="input" placeholder="filter by CN…" bind:value={filter} />
                <button class="btn" on:click={loadList} title="refresh">↻</button>
            </div>

            {#if err}<div class="muted">{err}</div>{/if}

            <ul class="list">
                {#each filteredList as x}
                    <li class="list-item" class:active={x.cn === cn} on:click={() => openCn(x)}>
                        <div class="cn">{x.cn}</div>
                        <div class="meta muted">
                            <span>{((x.size ?? 0) / 1024).toFixed(1)} KB</span>
                            <span>·</span>
                            <span>{new Date((x.modified ?? 0) * 1000).toLocaleString()}</span>
                        </div>
                    </li>
                {/each}

                {#if !filteredList.length}
                    <li class="muted">No CCDs</li>
                {/if}
            </ul>

        </aside>

        <main class="card grid">
            <div class="row">
                <input class="input" placeholder="CN…" bind:value={cn}
                       on:change={() => { if (cn) goto(`/ccd/${encodeURIComponent(cn)}`) }} />
                <button class="btn" on:click={load} disabled={!cn || loading}>{loading ? 'Loading…' : 'Load'}</button>
                <button class="btn primary" on:click={save} disabled={!cn || saving}>{saving ? 'Saving…' : 'Save'}</button>
            </div>

            {#if msg}<div class="muted">{msg}</div>{/if}
            {#if err}<div class="chip danger">Error: {err}</div>{/if}

            <textarea class="input mono" rows="18" bind:value={content}
                      placeholder={'e.g.\nifconfig-push 10.10.10.10 255.255.255.0\npush "redirect-gateway def1"'}></textarea>
        </main>
    </div>
</section>