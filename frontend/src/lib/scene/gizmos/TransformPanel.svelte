<script lang="ts">
	// Panneau numérique de transformation (T052) : saisie précise de la position
	// (mm), rotation (°) et échelle de l'objet sélectionné, plus « poser à plat ».
	// Émet la nouvelle transformation ; le parent applique et historise.
	// Audit 40 B3 : échelle en % (Orca « Scale ratios », 100 %) et ligne Taille
	// en mm, verrou d'échelle uniforme fermé par défaut (Orca m_uniform_scale).
	import { normalizeAngle, type Transform } from '../transform';
	import { fmtDec, locale, t } from '$lib/i18n';
	import IconLock from '~icons/lucide/lock';
	import IconUnlock from '~icons/lucide/lock-open';
	import {
		scaleFromPercent,
		scaleFromSize,
		scalePercent,
		scaledSize,
		type Vec3
	} from './scale-fields';

	interface Props {
		transform: Transform;
		onchange: (t: Transform) => void;
		/** Taille du maillage à l'échelle 1 (mm) : affiche la ligne Taille. */
		baseSize?: Vec3 | null;
		/** Pose à plat (le parent calcule la rotation depuis la facette choisie). */
		onlayflat?: () => void;
		/** Actif seulement si une facette est sélectionnée. */
		canLayFlat?: boolean;
		/**
		 * Audit 41 P1 : unité de longueur des Préférences (Orca ObjectManipulation,
		 * m_imperial_units) : position et taille affichées et saisies en pouces.
		 */
		units?: 'mm' | 'in';
	}

	let {
		transform,
		onchange,
		baseSize = null,
		onlayflat,
		canLayFlat = false,
		units = 'mm'
	}: Props = $props();
	/** mm → unité affichée (Orca : in = mm / 25,4, 4 décimales arrondies). */
	const factor = $derived(units === 'in' ? 1 / 25.4 : 1);
	const toShown = (mm: number) => (factor === 1 ? mm : Math.round(mm * factor * 10000) / 10000);

	type Field = 'position' | 'rotation' | 'scale' | 'size';
	// `name` : nom accessible des champs (« Échelle X », inchangé) ; `label` : libellé
	// visible avec l'unité.
	const rows = $derived(
		(
			[
				{
					field: 'position',
					label: units === 'in' ? $t('Position (in)') : $t('Position (mm)'),
					name: $t('Position (mm)'),
					step: units === 'in' ? 0.1 : 1
				},
				{ field: 'rotation', label: $t('Rotation (°)'), name: $t('Rotation (°)'), step: 1 },
				{ field: 'scale', label: $t('Scale (%)'), name: $t('Scale||transform'), step: 1 },
				{
					field: 'size',
					label: units === 'in' ? $t('Size (in)') : $t('Size (mm)'),
					name: $t('Size||transform'),
					step: units === 'in' ? 0.01 : 0.1
				}
			] as { field: Field; label: string; name: string; step: number }[]
		).filter((r) => r.field !== 'size' || !!baseSize)
	);
	const axes = ['X', 'Y', 'Z'];
	/** Verrou de proportions (Orca : échelle uniforme, fermé par défaut). */
	let uniform = $state(true);

	function shown(field: Field, i: number): number {
		if (field === 'scale') return scalePercent(transform.scale[i]);
		if (field === 'size')
			return baseSize ? toShown(scaledSize(baseSize, transform.scale as Vec3)[i]) : 0;
		if (field === 'position') return toShown(transform.position[i]);
		return transform[field][i];
	}

	/** Audit 42 V-b : en pouces, longueurs affichées avec le séparateur décimal
	 *  de la langue (« 0,7874 » en français, comme les champs d'Orca) : champ
	 *  texte décimal au lieu d'un champ number (point imposé par le navigateur). */
	const localized = (field: Field) => units === 'in' && (field === 'position' || field === 'size');
	// Phase 46 (audit 44 X-c) : rotation affichée à 2 décimales comme Orca
	// (ObjectManipulation : double_to_string(v, 2)) — affichage seulement, la
	// valeur exacte est gardée ; pas d'arrondi pendant la saisie du champ.
	let editing = $state<{ key: string; typed: number | null } | null>(null);
	function display(field: Field, i: number): string | number {
		const v = shown(field, i);
		if (field === 'rotation') {
			const typed = editing?.key === `rotation-${i}` ? editing.typed : null;
			return typed ?? (Math.round(v * 100) / 100 || 0);
		}
		return localized(field) ? fmtDec(v, 4, $locale) : v;
	}

	function setAxis(field: Field, index: number, raw: string) {
		// Saisie texte (pouces) : on n'applique qu'un nombre complet (« 0, » en
		// cours de frappe reste affiché tel quel).
		if (localized(field) && !/^\s*-?\d*(?:[.,]\d+)?\s*$/.test(raw)) return;
		const typed = Number(raw.replace(',', '.'));
		if (!Number.isFinite(typed) || raw.trim() === '') return;
		// Longueurs saisies dans l'unité affichée, appliquées en mm.
		const n = field === 'position' || field === 'size' ? typed / factor : typed;
		const next: Transform = {
			position: [...transform.position],
			rotation: [...transform.rotation],
			scale: [...transform.scale]
		};
		if (field === 'position') next.position[index] = n;
		else if (field === 'rotation') next.rotation[index] = normalizeAngle(n);
		else {
			const scale =
				field === 'scale'
					? scaleFromPercent(transform.scale as Vec3, index, n, uniform)
					: baseSize
						? scaleFromSize(transform.scale as Vec3, baseSize, index, n, uniform)
						: null;
			if (!scale) return;
			next.scale = scale;
		}
		onchange(next);
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	{#each rows as row (row.field)}
		<!-- Libellé + 3 champs ; l'axe est un préfixe dans le champ, la valeur
		     dispose de toute la largeur (« -123.45 » sans troncature). -->
		<div class="grid grid-cols-[4rem_repeat(3,minmax(0,1fr))] items-center gap-1.5">
			<span class="flex items-center gap-0.5 text-xs leading-tight text-content-muted"
				>{row.label}{#if row.field === 'scale'}<button
						type="button"
						class="ws-btn ws-btn-ghost -my-1 h-5 w-5 shrink-0 p-0 {uniform ? 'text-primary' : ''}"
						aria-pressed={uniform}
						aria-label={$t('Uniform scale')}
						title={uniform
							? $t('Uniform scale: proportions locked (click to scale each axis separately)')
							: $t('Uniform scale: off (click to lock the proportions)')}
						data-testid="transform-uniform-scale"
						onclick={() => (uniform = !uniform)}
						>{#if uniform}<IconLock class="size-3" aria-hidden="true" />{:else}<IconUnlock
								class="size-3"
								aria-hidden="true"
							/>{/if}</button
					>{/if}</span
			>
			{#each axes as axis, i (axis)}
				<label class="relative flex min-w-0 items-center">
					<span
						class="pointer-events-none absolute left-1.5 text-2xs font-semibold text-content-subtle"
						aria-hidden="true">{axis}</span
					>
					<input
						type={localized(row.field) ? 'text' : 'number'}
						inputmode={localized(row.field) ? 'decimal' : undefined}
						step={localized(row.field) ? undefined : row.step}
						value={display(row.field, i)}
						oninput={(e) => {
							if (row.field === 'rotation' && e.currentTarget.value.trim() !== '')
								editing = { key: `rotation-${i}`, typed: Number(e.currentTarget.value) };
							setAxis(row.field, i, e.currentTarget.value);
						}}
						onfocus={() => (editing = { key: `${row.field}-${i}`, typed: null })}
						onblur={() => (editing = null)}
						class="ws-field ws-field-sm w-full min-w-0 pr-1.5 pl-4 text-right"
						aria-label={`${row.name} ${axis}`}
						data-testid={`transform-${row.field}-${axis.toLowerCase()}`}
					/>
				</label>
			{/each}
		</div>
	{/each}

	<button
		type="button"
		class="ws-btn ws-btn-secondary mt-1 self-start"
		disabled={!canLayFlat}
		onclick={() => onlayflat?.()}
	>
		{$t('Lay on face')}
	</button>
</div>
