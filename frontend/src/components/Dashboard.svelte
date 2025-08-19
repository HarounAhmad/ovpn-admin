<script lang="ts">
    import { session } from '../lib/store'
</script>

<section style="padding:16px">
    <h2>Dashboard</h2>
    <pre>{JSON.stringify($session, null, 2)}</pre>
</section>
