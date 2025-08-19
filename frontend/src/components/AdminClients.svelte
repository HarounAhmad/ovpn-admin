<script lang="ts">
    import { postJson, postBlob } from '../lib/api'
    import { toast } from '../lib/toast'
    let cn=''; let includeKey=true; let busy=false; let last=''

    async function createClient(){
        if(!cn) return
        busy=true
        try{
            await postJson('/admin/clients',{cn,include_key:includeKey})
            last = cn
            toast('Client created')
        }catch(e){
            const m = (e as Error).message
            if (m.includes('409')) toast('Client exists')
            else toast(`Create failed: ${m}`)
        }finally{ busy=false }
    }

    async function downloadBundle(name = cn){
        if(!name) return
        busy=true
        try{
            const b=await postBlob(`/admin/clients/${encodeURIComponent(name)}/bundle`,{include_key:includeKey})
            const url=URL.createObjectURL(b)
            const a=document.createElement('a'); a.href=url; a.download=`${name}.zip`; a.click()
            URL.revokeObjectURL(url)
            toast('Bundle downloaded')
        }catch(e){ toast(`Bundle failed: ${(e as Error).message}`) }finally{ busy=false }
    }

    async function revoke(){
        if(!cn) return
        busy=true
        try{ await postJson(`/admin/clients/${encodeURIComponent(cn)}/revoke`,{}); toast('Client revoked') }
        catch(e){ toast(`Revoke failed: ${(e as Error).message}`) }
        finally{ busy=false }
    }

    function copy(v:string){ navigator.clipboard.writeText(v); toast('Copied') }
</script>

<section class="section" style="max-width:640px;display:grid;gap:12px">
    <h2>Clients</h2>
    <div style="display:grid;gap:8px">
        <label>Common Name <input class="input" bind:value={cn} placeholder="alice-laptop" /></label>
        <label><input type="checkbox" bind:checked={includeKey} /> include private key in bundle</label>
        <div style="display:flex;gap:8px;flex-wrap:wrap">
            <button class="btn primary" on:click={createClient} disabled={!cn||busy}>Create</button>
            <button class="btn" on:click={()=>downloadBundle(cn)} disabled={!cn||busy}>Bundle</button>
            <button class="btn" on:click={revoke} disabled={!cn||busy}>Revoke</button>
        </div>
    </div>

    {#if last}
        <div style="border:1px solid var(--border);border-radius:8px;padding:10px;display:flex;gap:8px;align-items:center">
            <div class="muted">Last created:</div>
            <code>{last}</code>
            <button class="btn" on:click={()=>copy(last)}>Copy</button>
            <button class="btn" on:click={()=>downloadBundle(last)} disabled={busy}>Download bundle</button>
        </div>
    {/if}

    {#if busy}<div>Working…</div>{/if}
</section>
