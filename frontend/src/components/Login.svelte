<script lang="ts">
    import { login } from '../lib/api'
    import { refreshSession } from '../lib/store'
    let username = ''
    let password = ''
    let err = ''
    async function submit(e: Event) {
        e.preventDefault()
        err = ''
        try {
            await login(username, password)
            await refreshSession()
            location.hash = '/'
        } catch (e) {
            err = (e as Error).message || 'login failed'
        }
    }
</script>

<form on:submit|preventDefault={submit} style="max-width:340px;margin:64px auto;display:flex;flex-direction:column;gap:10px">
    <h2>Sign in</h2>
    {#if err}<div style="color:#b00">{err}</div>{/if}
    <label>Username <input class="input" bind:value={username} required /></label>
    <label>Password <input class="input" type="password" bind:value={password} required /></label>
    <button class="btn" type="submit">Login</button>
</form>
