<script lang="ts">
    import { toasts } from '../lib/toast'
</script>
<div style="position:fixed;right:12px;bottom:12px;display:grid;gap:8px;z-index:9999">
    {#each $toasts as t}
        <div style="background:var(--text);color:var(--bg);padding:8px 12px;border-radius:6px;opacity:.95">{t.text}</div>
    {/each}
</div>
