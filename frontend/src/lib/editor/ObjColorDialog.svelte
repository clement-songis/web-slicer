<script lang="ts">
	// Couleurs d'un OBJ importé (Orca : ObjColorDialog) : couleurs détectées par
	// le serveur, nombre de couleurs (fusion des plus proches), et pour chacune
	// le filament qui l'imprimera — nouveau (couleur reprise) ou existant.
	// « Sans les couleurs » garde l'objet sur un seul filament.
	import Dialog from '$lib/ui/Dialog.svelte';
	import { swatchStyle } from '$lib/ui/color';
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

<Dialog {open} label="Couleurs de l'OBJ" size="lg" onclose={onskip} data-testid="obj-color-dialog">
	{#snippet header()}
		<div class="flex min-w-0 flex-col gap-0.5">
			<h2 class="ws-dialog-title">Couleurs de l'OBJ</h2>
			<p class="text-xs text-content-subtle">
				« {filename} » contient {palette.length} couleur{palette.length > 1 ? 's' : ''} : choisissez le
				filament de chacune. La peinture multi-matériaux de l'objet suit ce choix.
			</p>
		</div>
	{/snippet}
	<div class="flex flex-col gap-4 text-sm">
		<div class="flex flex-wrap items-center gap-x-6 gap-y-2">
			<label class="flex items-center gap-2">
				<span class="text-content-muted">Nombre de couleurs</span>
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
			<div class="flex items-center gap-1" role="group" aria-label="Affectation rapide">
				<span class="mr-1 text-content-muted">Affecter</span>
				<button
					type="button"
					class="ws-btn ws-btn-sm {mode === 'append' ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					aria-pressed={mode === 'append'}
					data-testid="obj-color-append"
					disabled={roomLeft <= 0}
					onclick={() => {
						mode = 'append';
						targets = defaultTargets(groups, slotColours, 'append');
					}}>À de nouveaux filaments</button
				>
				<button
					type="button"
					class="ws-btn ws-btn-sm {mode === 'match' ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					aria-pressed={mode === 'match'}
					data-testid="obj-color-match"
					onclick={() => {
						mode = 'match';
						targets = defaultTargets(groups, slotColours, 'match');
					}}>Aux filaments les plus proches</button
				>
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
						aria-label={`Filament pour la couleur ${g.hex}`}
						data-testid="obj-color-target"
						value={targetValue(targets[i])}
						onchange={(e) => setTarget(i, e.currentTarget.value)}
					>
						<option value="new" disabled={roomLeft <= 0}>Nouveau filament ({g.hex})</option>
						{#each slotColours as c, s (s)}
							<option value={String(s)}>Filament {s + 1} ({c})</option>
						{/each}
					</select>
					<span
						class="h-6 w-6 shrink-0 rounded border border-border-strong"
						style={swatch(resultColour(i) ?? g.hex)}
						title="Couleur du filament retenu"
						data-testid="obj-color-result"
					></span>
				</li>
			{/each}
		</ul>
		<p class="text-xs text-content-subtle" data-testid="obj-color-summary">
			{plan.newColours.length} filament{plan.newColours.length > 1 ? 's' : ''} ajouté{plan
				.newColours.length > 1
				? 's'
				: ''} · {reused} existant{reused > 1 ? 's' : ''} réutilisé{reused > 1 ? 's' : ''} · {slotColours.length +
				plan.newColours.length} / {MAX_FILAMENTS} filaments
		</p>
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost mr-auto"
			data-testid="obj-color-skip"
			onclick={onskip}>Importer sans les couleurs</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="obj-color-apply"
			onclick={() => onapply(plan)}>Appliquer les couleurs</button
		>
	{/snippet}
</Dialog>
