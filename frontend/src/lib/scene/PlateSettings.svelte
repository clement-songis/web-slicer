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
		/** Valeurs des presets (sans surcharge du plateau) : affichées dans
		 *  l'option « Profil (…) », comme « Same as Global … » d'Orca. */
		inherited?: Record<string, unknown>;
	}

	let { plate, onsetting, inherited = {} }: Props = $props();

	/** « Profil (Par couche) » : valeur effective quand le plateau ne surcharge rien. */
	function profileLabel(key: string): string {
		let v = inherited[key];
		if (Array.isArray(v)) v = v[0];
		if (v === undefined || v === null || v === '') return 'Profil';
		const s = typeof v === 'boolean' ? (v ? '1' : '0') : String(v);
		const opt = options(key).find((o) => o.value === s);
		return opt ? $t('Profile ({value})', { value: opt.label }) : $t('Profile||plate');
	}

	// Valeurs d'énumération (types de plaque, séquence) traduites comme dans le
	// dialogue de réglages : dictionnaire chargé à la demande.
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});

	const FIELDS = [
		// Libellés : clés i18n anglaises, traduites au rendu.
		{ key: 'curr_bed_type', label: 'Bed type (override)' },
		{ key: 'print_sequence', label: 'Print sequence' },
		{ key: 'spiral_mode', label: 'Spiral vase' },
		// Audit 48 DLG48-a : Timelapse (coEnum Traditional/Smooth, comme le
		// dialogue « Réglages du plateau »).
		{ key: 'timelapse_type', label: 'Timelapse' }
	] as const;

	function options(key: string): { value: string; label: string }[] {
		const def = PARAMS[key];
		if (!def) return [];
		if (def.type === 'coBool')
			return [
				{ value: '1', label: $t('Yes') },
				{ value: '0', label: $t('No') }
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
	<legend class="px-1 text-xs font-semibold text-content-subtle">{$t('Plate settings')}</legend>
	<!-- Le type de plaque se règle déjà juste au-dessus (barre du plateau) :
	     sa surcharge n'apparaît ici que si elle existe (posée par le dialogue). -->
	{#each FIELDS.filter((f) => f.key !== 'curr_bed_type' || current(f.key) !== '') as field (field.key)}
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">{$t(field.label)}</span>
			<select
				class="ws-field ws-field-sm w-36 shrink-0"
				data-testid={`plate-setting-${field.key}`}
				value={current(field.key)}
				onchange={(e) => {
					const v = e.currentTarget.value;
					onsetting(field.key, v === '' ? undefined : v);
				}}
			>
				<option value="">{profileLabel(field.key)}</option>
				{#each options(field.key) as opt (opt.value)}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
		</label>
	{/each}
</fieldset>
