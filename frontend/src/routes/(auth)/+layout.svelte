<script lang="ts">
	let { children } = $props();
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50 p-4 dark:bg-gray-900">
	<div class="w-full max-w-sm rounded-lg bg-white p-8 shadow dark:bg-gray-800">
		{@render children()}
	</div>
</div>
