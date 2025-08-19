<script lang="ts">
    import { login } from '../lib/store'
    let username = ''
    let password = ''
    let err = ''
    async function submit() {
        err = ''
        try { await login(username, password) } catch (e) { err = String(e) }
    }
</script>

<section class="pad grid" style="max-width:420px;margin:10vh auto 0;">
    <h2>Sign in</h2>
    <input class="input" placeholder="username" bind:value={username} />
    <input class="input" type="password" placeholder="password" bind:value={password} />
    {#if err}<div class="muted">{err}</div>{/if}
    <button class="btn primary" on:click|preventDefault={submit}>Sign in</button>
</section>
