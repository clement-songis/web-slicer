<script lang="ts">
	// Réglages par plateau (parité Orca « Plate settings », PlateSettingsDialog) :
	// type de plaque, séquence d'impression, vase spirale. « Profil » = pas de
	// surcharge (le plateau suit les presets). Présentationnel.
	import { PARAMS } from '../../generated/params';
	import type { Plate } from './plates.svelte';
	import { locale, t } from '$lib/i18n';
	import { loadSettingsTranslations } from '$lib/settings/i18n';

	interface Props {
		plate: Plate;
		onsetting: (key: string, value: unknown) => void;
	}

	let { plate, onsetting }: Props = $props();

	// Valeurs d'énumération (types de plaque, séquence) traduites comme dans le
	// dialogue de réglages : dictionnaire chargé à la demande.
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});

	const FIELDS = [
		{ key: 'curr_bed_type', label: 'Type de plaque (surcharge)' },
		{ key: 'print_sequence', label: "Séquence d'impression" },
		{ key: 'spiral_mode', label: 'Vase spirale' }
	] as const;

	function options(key: string): { value: string; label: string }[] {
		const def = PARAMS[key];
		if (!def) return [];
		if (def.type === 'coBool')
			return [
				{ value: '1', label: 'Oui' },
				{ value: '0', label: 'Non' }
			];
		return def.enumValues.map((value, i) => ({ value, label: $t(def.enumLabels[i] || value) }));
	}

	function current(key: string): string {
		const v = plate.settings?.[key];
		if (v === undefined || v === null) return '';
		if (typeof v === 'boolean') return v ? '1' : '0';
		return String(v);
	}
</script>

<fieldset class="flex flex-col gap-1 rounded border border-border p-2" data-testid="plate-settings">
	<legend class="px-1 text-xs font-semibold text-content-subtle">Réglages du plateau</legend>
	<!-- Le type de plaque se règle déjà juste au-dessus (barre du plateau) :
	     sa surcharge n'apparaît ici que si elle existe (posée par le dialogue). -->
	{#each FIELDS.filter((f) => f.key !== 'curr_bed_type' || current(f.key) !== '') as field (field.key)}
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">{field.label}</span>
			<select
				class="ws-field ws-field-sm w-36 shrink-0"
				data-testid={`plate-setting-${field.key}`}
				value={current(field.key)}
				onchange={(e) => {
					const v = e.currentTarget.value;
					onsetting(field.key, v === '' ? undefined : v);
				}}
			>
				<option value="">Profil</option>
				{#each options(field.key) as opt (opt.value)}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
		</label>
	{/each}
</fieldset>
