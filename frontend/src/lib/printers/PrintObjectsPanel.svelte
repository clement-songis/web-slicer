<script lang="ts">
	// Objets de l'impression en cours (équivalent web du PartSkipDialog d'Orca,
	// via Klipper exclude_object) : vue de dessus des contours, liste avec état
	// (en cours / à imprimer / exclu) et « Exclure » pendant l'impression, avec
	// confirmation (irréversible). Section masquée si le serveur n'a pas la route.
	import { ApiError } from '$lib/api/client';
	import { excludePrintObject, getPrintObjects } from '$lib/api/printers';
	import type { ExcludeObjectStatus } from '$lib/api/types';
	import {
		isMissingRoute,
		objectsBounds,
		printObjectLabel,
		printObjectState,
		type PrintObjectState
	} from './device';
	import { t, tr } from '$lib/i18n';

	interface Props {
		printerId: string;
		/** Impression en cours ou en pause (exclusion possible). */
		active: boolean;
		/** Change à chaque rafraîchissement du statut (relance la lecture). */
		tick: number;
	}
	let { printerId, active, tick }: Props = $props();

	let status = $state<ExcludeObjectStatus | null>(null);
	let missing = $state(false);
	let error = $state<string | null>(null);
	let confirm = $state<string | null>(null);
	let busy = $state<string | null>(null);
	let hovered = $state<string | null>(null);

	$effect(() => {
		void tick;
		const id = printerId;
		if (missing) return;
		getPrintObjects(id)
			.then((s) => {
				if (id === printerId) status = s;
			})
			.catch((e) => {
				if (isMissingRoute(e)) missing = true;
			});
	});
	$effect(() => {
		void printerId;
		status = null;
		confirm = null;
		error = null;
	});

	async function exclude(name: string) {
		busy = name;
		error = null;
		try {
			status = await excludePrintObject(printerId, name);
			confirm = null;
		} catch (e) {
			error = e instanceof ApiError ? e.message : tr('exclusion rejected');
		} finally {
			busy = null;
		}
	}

	const objects = $derived(status?.objects ?? []);
	const bounds = $derived(objectsBounds(objects));
	const stateOf = (name: string): PrintObjectState =>
		status ? printObjectState(name, status) : 'pending';
	// Clés anglaises, traduites au rendu.
	const LABELS: Record<PrintObjectState, string> = {
		current: 'printing||object',
		pending: 'to print',
		excluded: 'excluded||one'
	};
	/** Nom lisible (« copie n » traduit, relu au changement de langue). */
	const labelOf = (name: string): string => {
		void $t;
		return printObjectLabel(name);
	};
	const excludedCount = $derived(objects.filter((o) => stateOf(o.name) === 'excluded').length);
	const flip = (y: number) => (bounds ? 2 * bounds.minY + bounds.height - y : y);
</script>

{#if !missing && (objects.length > 0 || active)}
	<section
		class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
		data-testid="device-objects"
	>
		<div class="flex items-center justify-between gap-2">
			<h3 class="ws-section-title">{$t('Print objects')}</h3>
			{#if objects.length}
				<span class="text-xs text-content-subtle"
					>{objects.length > 1
						? $t('{n} objects', { n: objects.length })
						: $t('{n} object', { n: objects.length })}{excludedCount
						? excludedCount > 1
							? $t(' · {n} excluded||many', { n: excludedCount })
							: $t(' · {n} excluded||one', { n: excludedCount })
						: ''}</span
				>
			{/if}
		</div>
		{#if !objects.length}
			<p class="text-sm text-content-muted" data-testid="device-objects-empty">
				{$t(
					'No labeled object in this G-code. To be able to exclude an object while printing, enable “Exclude objects” in the process settings (Others tab) before slicing, and the [exclude_object] section in the Klipper configuration.'
				)}
			</p>
		{:else}
			<div class="flex flex-col gap-3 sm:flex-row">
				{#if bounds}
					<svg
						class="h-40 w-full shrink-0 rounded border border-border bg-surface sm:w-48"
						viewBox="{bounds.minX} {bounds.minY} {bounds.width} {bounds.height}"
						role="img"
						aria-label={$t('Top view of the objects')}
						data-testid="device-objects-map"
					>
						{#each objects as o (o.name)}
							{@const s = stateOf(o.name)}
							{#if o.polygon.length}
								<polygon
									points={o.polygon.map(([x, y]) => `${x},${flip(y)}`).join(' ')}
									vector-effect="non-scaling-stroke"
									class="{s === 'excluded'
										? 'fill-danger/15 stroke-danger'
										: s === 'current'
											? 'fill-primary/40 stroke-primary'
											: 'fill-content/10 stroke-content-subtle'} {hovered === o.name
										? 'stroke-2'
										: ''}"
									stroke-dasharray={s === 'excluded' ? '4 3' : undefined}
								/>
							{/if}
						{/each}
					</svg>
				{/if}
				<ul class="flex min-w-0 flex-1 flex-col divide-y divide-border text-sm">
					{#each objects as o (o.name)}
						{@const s = stateOf(o.name)}
						<li
							class="flex items-center justify-between gap-2 py-1.5"
							data-testid="device-object"
							data-state={s}
							onmouseenter={() => (hovered = o.name)}
							onmouseleave={() => (hovered = null)}
						>
							<span class="min-w-0">
								<span
									class="block truncate {s === 'excluded'
										? 'text-content-subtle line-through'
										: 'text-content'}"
									title={o.name}>{labelOf(o.name)}</span
								>
								<span
									class="text-xs {s === 'current'
										? 'text-primary'
										: s === 'excluded'
											? 'text-danger-text'
											: 'text-content-subtle'}">{$t(LABELS[s])}</span
								>
							</span>
							{#if s !== 'excluded'}
								{#if confirm === o.name}
									<span class="flex shrink-0 items-center gap-1">
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-danger"
											data-testid="device-object-exclude-confirm"
											disabled={busy !== null}
											aria-busy={busy === o.name}
											onclick={() => exclude(o.name)}>{$t('Exclude permanently')}</button
										>
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-ghost"
											onclick={() => (confirm = null)}>{$t('Cancel')}</button
										>
									</span>
								{:else}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
										data-testid="device-object-exclude"
										disabled={!active || busy !== null}
										title={active
											? $t('Stop printing this object (the others continue)')
											: $t('Only possible during a print')}
										onclick={() => (confirm = o.name)}>{$t('Exclude')}</button
									>
								{/if}
							{/if}
						</li>
					{/each}
				</ul>
			</div>
			{#if confirm}
				<p class="text-xs text-warning-text" role="status">
					{$t(
						'The printer will stop printing “{name}” from the current layer. Irreversible action for this print.',
						{ name: printObjectLabel(confirm) }
					)}
				</p>
			{/if}
			{#if error}<p class="text-xs text-danger" role="alert">{error}</p>{/if}
		{/if}
	</section>
{/if}
