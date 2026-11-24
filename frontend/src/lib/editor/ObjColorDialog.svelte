<script lang="ts">
	// Couleurs d'un OBJ importé (Orca : ObjColorDialog) : couleurs détectées par
	// le serveur, nombre de couleurs (fusion des plus proches), et pour chacune
	// le filament qui l'imprimera — nouveau (couleur reprise) ou existant.
	// « Sans les couleurs » garde l'objet sur un seul filament.
	import Dialog from '#lib/ui/Dialog.svelte';
	import { swatchStyle } from '#lib/ui/color';
	import { t } from '#lib/i18n';
	import {
		MAX_FILAMENTS,
		defaultTargets,
		groupPalette,
		paletteHex,
		planObjColors,
		type ColorTarget,
		type ObjColorPlan
	} from './obj-colors';

	interface Props {
		open: boolean;
		filename: string;
		/** Palette du serveur (« #RRGGBBAA »). */
		palette: string[];
		/** Couleurs effectives des filaments du projet. */
		slotColours: string[];
		onapply: (plan: ObjColorPlan) => void;
		onskip: () => void;
	}

	let { open, filename, palette, slotColours, onapply, onskip }: Props = $props();

	let count = $state(1);
	let mode = $state<'append' | 'match'>('append');
	let lastOpen = false;
	$effect(() => {
		if (open && !lastOpen) {
			count = palette.length;
			mode = 'append';
		}
		lastOpen = open;
	});
	const groups = $derived(groupPalette(palette, count));
	// Affectation recalculée quand le nombre de couleurs ou le mode change
	// (dérivé modifiable : les sélecteurs de ligne la surchargent).
	let targets = $derived<ColorTarget[]>(defaultTargets(groups, slotColours, mode));
	const plan = $derived(planObjColors(palette, groups, targets, slotColours.length));
	const reused = $derived(
		new Set(targets.filter((t) => t.kind === 'slot').map((t) => (t as { slot: number }).slot)).size
	);
	const roomLeft = $derived(MAX_FILAMENTS - slotColours.length);

	function targetValue(t: ColorTarget | undefined): string {
		return !t || t.kind === 'new' ? 'new' : String(t.slot);
	}
	function setTarget(i: number, v: string) {
		const next = [...targets];
		next[i] = v === 'new' ? { kind: 'new' } : { kind: 'slot', slot: Number(v) };
		targets = next;
	}
	/** Couleur du filament qui imprimera le groupe `i`. */
	function resultColour(i: number): string {
		const g = groups[i];
		const ex = plan.extruderOf[g.members[0]] ?? 1;
		return ex > slotColours.length
			? plan.newColours[ex - slotColours.length - 1]
			: slotColours[ex - 1];
	}
	const swatch = (c: string) => swatchStyle(paletteHex(c) ?? c);
</script>

<Dialog {open} label={$t('OBJ colors')} size="lg" onclose={onskip} data-testid="obj-color-dialog">
	{#snippet header()}
		<div class="flex min-w-0 flex-col gap-0.5">
			<h2 class="ws-dialog-title">{$t('OBJ colors')}</h2>
			<p class="text-xs text-content-subtle">
				{$t(
					palette.length > 1
						? "“{file}” contains {n} colors: choose the filament of each. The object's multi-material painting follows this choice."
						: "“{file}” contains {n} color: choose the filament of each. The object's multi-material painting follows this choice.",
					{ file: filename, n: palette.length }
				)}
			</p>
		</div>
	{/snippet}
	<div class="flex flex-col gap-4 text-sm">
		<div class="flex flex-wrap items-center gap-x-6 gap-y-2">
			<label class="flex items-center gap-2">
				<span class="text-content-muted">{$t('Number of colors')}</span>
				<input
					type="number"
					class="ws-field ws-field-sm w-16 text-right"
					min="1"
					max={palette.length}
					value={count}
					data-testid="obj-color-count"
					onchange={(e) =>
						(count = Math.max(1, Math.min(palette.length, Number(e.currentTarget.value) || 1)))}
				/>
			</label>
			<div class="flex items-center gap-1" role="group" aria-label={$t('Quick assignment')}>
				<span class="mr-1 text-content-muted">{$t('Assign')}</span>
				<button
					type="button"
					class="ws-btn ws-btn-sm {mode === 'append' ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					aria-pressed={mode === 'append'}
					data-testid="obj-color-append"
					disabled={roomLeft <= 0}
					onclick={() => {
						mode = 'append';
						targets = defaultTargets(groups, slotColours, 'append');
					}}
				>
					{$t('To new filaments')}
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-sm {mode === 'match' ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					aria-pressed={mode === 'match'}
					data-testid="obj-color-match"
					onclick={() => {
						mode = 'match';
						targets = defaultTargets(groups, slotColours, 'match');
					}}
				>
					{$t('To the closest filaments')}
				</button>
			</div>
		</div>

		<ul class="divide-y divide-border rounded border border-border" data-testid="obj-color-rows">
			{#each groups as g, i (g.members.join('-'))}
				<li class="flex items-center gap-3 px-3 py-2" data-testid="obj-color-row">
					<span class="flex shrink-0 -space-x-1">
						{#each g.members as m (m)}
							<span
								class="h-6 w-6 rounded-full border border-border-strong"
								style={swatch(palette[m])}
								title={paletteHex(palette[m])}
							></span>
						{/each}
					</span>
					<span class="w-20 font-mono text-xs text-content-subtle">{g.hex}</span>
					<span aria-hidden="true" class="text-content-subtle">→</span>
					<select
						class="ws-field ws-field-sm min-w-0 flex-1"
						aria-label={$t('Filament for color {hex}', { hex: g.hex })}
						data-testid="obj-color-target"
						value={targetValue(targets[i])}
						onchange={(e) => setTarget(i, e.currentTarget.value)}
					>
						<option value="new" disabled={roomLeft <= 0}>
							{$t('New filament ({hex})', { hex: g.hex })}
						</option>
						{#each slotColours as c, s (s)}
							<option value={String(s)}>
								{$t('Filament {n} ({color})', { n: s + 1, color: c })}
							</option>
						{/each}
					</select>
					<span
						class="h-6 w-6 shrink-0 rounded border border-border-strong"
						style={swatch(resultColour(i) ?? g.hex)}
						title={$t('Color of the chosen filament')}
						data-testid="obj-color-result"
					></span>
				</li>
			{/each}
		</ul>
		<p class="text-xs text-content-subtle" data-testid="obj-color-summary">
			{$t(plan.newColours.length > 1 ? '{n} filaments added' : '{n} filament added', {
				n: plan.newColours.length
			})} · {$t(reused > 1 ? '{n} existing reused||plural' : '{n} existing reused', { n: reused })} ·
			{$t('{used} / {max} filaments', {
				used: slotColours.length + plan.newColours.length,
				max: MAX_FILAMENTS
			})}
		</p>
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost mr-auto"
			data-testid="obj-color-skip"
			onclick={onskip}
		>
			{$t('Import without colors')}
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="obj-color-apply"
			onclick={() => onapply(plan)}
		>
			{$t('Apply colors')}
		</button>
	{/snippet}
</Dialog>
