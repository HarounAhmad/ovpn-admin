<script lang="ts">
    import {postJson, postBlob} from '../lib/api'
    import {toast} from '../lib/toast'

    let cn = '';
    let includeKey = true;
    let busy = false

    async function createClient() {
        if (!cn) return;
        busy = true;
        try {
            await postJson('/admin/clients', {cn, include_key: includeKey});
            toast('Client created')
        } catch (e) {
            toast(`Create failed: ${(e as Error).message}`)
        } finally {
            busy = false
        }
    }

    async function downloadBundle() {
        if (!cn) return;
        busy = true;
        try {
            const b = await postBlob(`/admin/clients/${encodeURIComponent(cn)}/bundle`, {include_key: includeKey});
            const url = URL.createObjectURL(b);
            const a = document.createElement('a');
            a.href = url;
            a.download = `${cn}.zip`;
            a.click();
            URL.revokeObjectURL(url);
            toast('Bundle downloaded')
        } catch (e) {
            toast(`Bundle failed: ${(e as Error).message}`)
        } finally {
            busy = false
        }
    }

    async function revoke() {
        if (!cn) return;
        busy = true;
        try {
            await postJson(`/admin/clients/${encodeURIComponent(cn)}/revoke`, {});
            toast('Client revoked')
        } catch (e) {
            toast(`Revoke failed: ${(e as Error).message}`)
        } finally {
            busy = false
        }
    }
</script>

<section class="section" style="max-width:520px;display:grid;gap:12px">
    <h2>Clients</h2>
    <label>Common Name <input class="input" bind:value={cn} placeholder="alice-laptop"/></label>
    <label><input type="checkbox" bind:checked={includeKey}/> include private key in bundle</label>
    <div style="display:flex;gap:8px">
        <button class="btn primary" on:click={createClient} disabled={!cn||busy}>Create</button>
        <button class="btn" on:click={downloadBundle} disabled={!cn||busy}>Bundle</button>
        <button class="btn" on:click={revoke} disabled={!cn||busy}>Revoke</button>
    </div>
    {#if busy}
        <div>Working…</div>
    {/if}
</section>
