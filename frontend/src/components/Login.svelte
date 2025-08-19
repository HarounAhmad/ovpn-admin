<script lang="ts">
    import { login } from '../lib/api'
    import { refreshSession } from '../lib/store'
    let username = ''
    let password = ''
    let busy = false
    let err = ''

    async function submit(e: Event) {
        e.preventDefault()
        if (!username || !password) return
        busy = true; err = ''
        try {
            await login(username, password)
            await refreshSession()
        } catch (e) {
            err = (e as Error).message || 'login failed'
        } finally {
            busy = false
        }
    }
</script>

<section class="section" style="max-width:420px;margin:40px auto;display:grid;gap:12px">
    <h2>Login</h2>
    {#if err}<div style="color:#b00020">{err}</div>{/if}
    <form on:submit|preventDefault={submit} style="display:grid;gap:10px">
        <label>Username <input class="input" bind:value={username} autocomplete="username" /></label>
        <label>Password <input class="input" type="password" bind:value={password} autocomplete="current-password" /></label>
        <div style="display:flex;gap:8px">
            <button class="btn primary" disabled={busy || !username || !password} on:click={submit}>Sign in</button>
            <button class="btn" type="reset" on:click={() => { username=''; password=''; err='' }} disabled={busy}>Clear</button>
        </div>
    </form>
</section>
