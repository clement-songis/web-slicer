<script lang="ts">
	let { children } = $props();
</script>

<div class="flex min-h-screen items-center justify-center bg-surface p-4">
	<div class="w-full max-w-sm rounded-lg bg-surface-raised p-8 shadow">
		{@render children()}
	</div>
</div>
