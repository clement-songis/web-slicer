<script lang="ts">
	// Barre de notifications de validation (phase 29), comme celle d'Orca en
	// bas à droite de la vue 3D (NotificationManager : « Erreur : » rouge,
	// « Avertissement : » orange, lien « Jump to [objet] (option) »). Plus
	// lisible qu'Orca : liens séparés (objet / réglage / plateau), action
	// directe « Disposer » pour les collisions, résumé repliable, avertissements
	// masquables un par un (ils reviennent si leur texte change).
	import IconError from '~icons/lucide/circle-alert';
	import IconWarning from '~icons/lucide/triangle-alert';
	import IconChevronDown from '~icons/lucide/chevron-down';
	import IconChevronUp from '~icons/lucide/chevron-up';
	import IconX from '~icons/lucide/x';
	import IconCrosshair from '~icons/lucide/crosshair';
	import IconSliders from '~icons/lucide/sliders-horizontal';
	import IconGrid from '~icons/lucide/layout-grid';
	import { locale, t } from '$lib/i18n';
	import { noticeSummary, type ValidationNotice } from './validation';

	interface Props {
		notices: readonly ValidationNotice[];
		/** Validation en cours (après une modification, avec délai). */
		busy?: boolean;
		/** Validation impossible (conflit, erreur réseau) : message affiché. */
		blocked?: string | null;
		/** Plateau affiché (0-based) : un lien « Plateau N » mène aux autres. */
		activePlate?: number;
		ongotoobject?: (objectId: string) => void;
		ongotooption?: (notice: ValidationNotice) => void;
		ongotoplate?: (plateIndex: number) => void;
		onarrange?: () => void;
	}

	let {
		notices,
		busy = false,
		blocked = null,
		activePlate = 0,
		ongotoobject,
		ongotooption,
		ongotoplate,
		onarrange
	}: Props = $props();

	let collapsed = $state(false);
	let dismissed = $state<string[]>([]);
	const shown = $derived(
		notices.filter((n) => n.level === 'error' || !dismissed.includes(n.level + n.text))
	);
	const errors = $derived(shown.filter((n) => n.level === 'error').length);
	// Nouvelle erreur : le panneau se déplie (Orca pousse la notification).
	let lastErrors = 0;
	$effect(() => {
		if (errors > lastErrors) collapsed = false;
		lastErrors = errors;
	});
</script>

{#if shown.length || blocked}
	<section
		class="ws-floating flex w-[min(26rem,calc(100vw-2rem))] flex-col overflow-hidden text-xs text-content"
		data-testid="validation-panel"
		data-errors={errors}
		data-warnings={shown.length - errors}
		data-busy={busy ? 'true' : undefined}
		aria-label={$t('Pre-slicing check')}
	>
		<button
			type="button"
			class="flex items-center gap-2 px-3 py-2 text-left font-medium hover:bg-overlay"
			data-testid="validation-toggle"
			aria-expanded={!collapsed}
			onclick={() => (collapsed = !collapsed)}
		>
			<span
				class="h-2 w-2 shrink-0 rounded-full {errors ? 'bg-danger' : 'bg-warning'}"
				aria-hidden="true"
			></span>
			<span class="min-w-0 flex-1 truncate" role="status" aria-live="polite">
				{blocked && !shown.length ? $t('Check suspended') : $locale && noticeSummary(shown)}
				{#if errors}<span class="font-normal text-content-muted">{$t(' — slicing impossible')}</span
					>{/if}
			</span>
			{#if busy}
				<span
					class="inline-block size-3 shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent text-content-muted"
					data-testid="validation-busy"
					aria-label={$t('Checking')}
				></span>
			{/if}
			{#if collapsed}<IconChevronUp
					class="h-4 w-4 shrink-0"
					aria-hidden="true"
				/>{:else}<IconChevronDown class="h-4 w-4 shrink-0" aria-hidden="true" />{/if}
		</button>
		{#if !collapsed}
			<ul class="flex max-h-72 flex-col gap-1.5 overflow-y-auto border-t border-border p-2">
				{#if blocked}
					<li
						class="rounded-md bg-surface-sunken px-2 py-1.5 text-content-muted"
						data-testid="validation-blocked"
					>
						{blocked}
					</li>
				{/if}
				{#each shown as n (n.id + n.text)}
					<li
						class="flex gap-2 rounded-md border-l-4 px-2 py-1.5 {n.level === 'error'
							? 'border-danger bg-danger-soft text-danger-content'
							: 'border-warning bg-warning-soft text-warning-content'}"
						data-testid="validation-notice"
						data-level={n.level}
						data-kind={n.kind}
						data-object={n.objectId}
						data-opt={n.optKey}
						data-plate={n.plateIndex ?? undefined}
					>
						{#if n.level === 'error'}
							<IconError class="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
						{:else}
							<IconWarning class="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
						{/if}
						<div class="flex min-w-0 flex-1 flex-col gap-1">
							<p class="leading-snug break-words whitespace-pre-line">
								<span class="sr-only">{n.level === 'error' ? $t('Error: ') : $t('Warning: ')}</span
								>{n.text}
							</p>
							{#if (n.objectId && ongotoobject) || (n.optKey && ongotooption) || (n.action && onarrange) || (n.plateIndex !== null && n.plateIndex !== activePlate && ongotoplate)}
								<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
									{#if n.plateIndex !== null && n.plateIndex !== activePlate && ongotoplate}
										<button
											type="button"
											class="ws-link inline-flex items-center gap-1 font-medium"
											data-testid="validation-goto-plate"
											onclick={() => ongotoplate(n.plateIndex!)}
											><IconGrid class="h-3 w-3" aria-hidden="true" />{$t('View plate {n}', {
												n: n.plateIndex! + 1
											})}</button
										>
									{/if}
									{#if n.objectId && ongotoobject}
										<button
											type="button"
											class="ws-link inline-flex max-w-full items-center gap-1 truncate font-medium"
											data-testid="validation-goto-object"
											onclick={() => ongotoobject(n.objectId!)}
											><IconCrosshair class="h-3 w-3 shrink-0" aria-hidden="true" />{$t(
												'Select “{name}”',
												{
													name: n.objectName ?? $t('object')
												}
											)}</button
										>
									{/if}
									{#if n.optKey && ongotooption}
										<button
											type="button"
											class="ws-link inline-flex items-center gap-1 font-medium"
											data-testid="validation-goto-option"
											title={n.optKey}
											onclick={() => ongotooption(n)}
											><IconSliders class="h-3 w-3 shrink-0" aria-hidden="true" />{$t(
												'Adjust “{name}”',
												{
													name: n.optLabel ? $t(n.optLabel) : (n.optKey ?? '')
												}
											)}</button
										>
									{/if}
									{#if n.action === 'arrange' && onarrange}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											data-testid="validation-arrange"
											onclick={() => onarrange()}>{$t('Arrange')}</button
										>
									{/if}
								</div>
							{/if}
						</div>
						{#if n.level === 'warning'}
							<button
								type="button"
								class="ws-icon-btn ws-icon-btn-xs shrink-0 self-start"
								data-testid="validation-dismiss"
								aria-label={$t('Hide this warning')}
								onclick={() => (dismissed = [...dismissed, n.level + n.text])}
							>
								<IconX class="h-3 w-3" aria-hidden="true" />
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</section>
{/if}
