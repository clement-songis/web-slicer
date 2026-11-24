<script lang="ts">
	// Fenêtre « position de l'outil » de l'aperçu (Orca GCodeViewer, fenêtre
	// ToolPosition en bas de la vue) : X / Y / Z du sommet courant, vitesse et
	// détail de la coloration active ; propriétés dépliables (type de ligne,
	// largeur, hauteur, couche, vitesse, accélération, jerk, débit, ventilateur,
	// température, avance de pression, outil, temps de couche). N/D si la grandeur
	// n'est pas connue (début de couche, piste G-code non chargée).
	import { locale, numberLocale, t } from '#lib/i18n';
	import {
		SEGMENT_ROLE_COLORS,
		SEGMENT_ROLE_NAMES,
		type Coloration,
		type Rgb
	} from './colorations';
	import { formatDuration } from './stats';
	import type { ToolPosition } from './tool-position';
	import type { EstimatedTime } from './engine-data';

	interface Props {
		tool: ToolPosition;
		coloration: Coloration;
		/** Temps estimé au sommet (simulation Orca, lot 31) ; null = inconnu. */
		time?: EstimatedTime | null;
	}
	let { tool, coloration, time = null }: Props = $props();
	let open = $state(false);

	const NA = $derived($t('N/A||tool'));
	const num = (v: number | null | undefined, digits: number, unit = '') =>
		v === null || v === undefined || !Number.isFinite(v)
			? NA
			: `${v.toLocaleString(numberLocale($locale), { minimumFractionDigits: digits, maximumFractionDigits: digits })}${unit ? ' ' + unit : ''}`;
	// « Temps : 1h2m (0.123s) » comme get_estimated_time_at d'Orca.
	const timeText = $derived(
		time
			? formatDuration(time.time) +
					(time.moveTime !== null ? ` (${num(time.moveTime, 3, 's')})` : '')
			: NA
	);
	const css = ([r, g, b]: Rgb) =>
		`rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
	const roleName = $derived(
		tool.kind === null ? null : $t(SEGMENT_ROLE_NAMES[tool.kind] ?? `#${tool.kind}`)
	);

	// Détail selon la coloration (comme detail_buf d'Orca).
	const detail = $derived.by((): string => {
		const ch = tool.channels;
		switch (coloration) {
			case 'type':
				return roleName ?? $t('Travel');
			case 'height':
				return $t('Height: {value}', { value: num(tool.height, 2, 'mm') });
			case 'width':
				return $t('Width: {value}', { value: num(tool.width, 2, 'mm') });
			case 'flow':
				return $t('Flow: {value}', { value: num(tool.flow, 2, 'mm³/s') });
			case 'actual-flow':
				return $t('Actual flow: {value}', { value: num(ch.actualFlow, 2, 'mm³/s') });
			case 'actual-speed':
				return $t('Actual speed: {value}', { value: num(ch.actualSpeed, 1, 'mm/s') });
			case 'fan':
				return $t('Fan: {value}', { value: num(ch.fan, 0, '%') });
			case 'temperature':
				return $t('Temperature: {value}', { value: num(ch.temperature, 0, '°C') });
			case 'layer-time':
			case 'layer-time-log':
				return $t('Layer time: {value}', {
					value: tool.layerTime === null ? NA : formatDuration(tool.layerTime)
				});
			case 'filament':
				return tool.extruder === null ? NA : $t('Filament {n}', { n: tool.extruder + 1 });
			case 'acceleration':
				return $t('Acceleration: {value}', { value: num(ch.acceleration, 0, 'mm/s²') });
			case 'jerk':
				return $t('Jerk: {value}', { value: num(ch.jerk, 1, 'mm/s') });
			case 'pa':
				return $t('PA: {value}', { value: num(ch.pa, 4) });
			default:
				return '';
		}
	});

	const rows = $derived<[string, string][]>([
		// Libellés = clés i18n anglaises (traduites au rendu).
		['Type', tool.kind === null ? $t('Layer start') : $t('Extrusion')],
		['Line type', roleName ?? NA],
		['Width', num(tool.width, 3, 'mm')],
		['Height', num(tool.height, 3, 'mm')],
		['Layer', String(tool.layer + 1)],
		['Speed', num(tool.speed, 1, 'mm/s')],
		['Acceleration', num(tool.channels.acceleration, 0, 'mm/s²')],
		['Jerk', num(tool.channels.jerk, 1, 'mm/s')],
		['Flow', num(tool.flow, 3, 'mm³/s')],
		['Fan', num(tool.channels.fan, 0, '%')],
		['Temperature', num(tool.channels.temperature, 0, '°C')],
		['Pressure advance||tool', num(tool.channels.pa, 4)],
		['Tool', tool.extruder === null ? NA : String(tool.extruder + 1)],
		['Layer time', tool.layerTime === null ? NA : formatDuration(tool.layerTime)],
		['Estimated time', timeText]
	]);
</script>

<div
	class="ws-floating flex flex-col gap-1 px-2.5 py-1.5 text-xs text-content-muted"
	data-testid="preview-tool-position"
	data-x={tool.position[0].toFixed(3)}
	data-y={tool.position[1].toFixed(3)}
	data-z={tool.position[2].toFixed(3)}
	data-index={tool.index}
	role="group"
	aria-label={$t('Tool position')}
>
	{#if open}
		<dl
			class="grid grid-cols-[auto_auto] gap-x-4 gap-y-0.5 border-b border-border pb-1.5 tabular-nums"
			data-testid="preview-tool-properties"
		>
			{#each rows as [label, value] (label)}
				<dt class="text-content-subtle">{$t(label)}</dt>
				<dd class="text-right text-content">{value}</dd>
			{/each}
		</dl>
	{/if}
	<!-- Phase 44b : les groupes insécables passent à la ligne quand la vue est
	     étroite (la ligne débordait de la fenêtre à 760 × 490). -->
	<div class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
		<button
			type="button"
			class="ws-icon-btn ws-icon-btn-xs"
			aria-expanded={open}
			aria-label={open ? $t('Hide properties') : $t('Show properties')}
			data-tip={open ? $t('Hide properties') : $t('Properties of the current point')}
			data-tip-side="top"
			data-testid="preview-tool-toggle"
			onclick={() => (open = !open)}
		>
			{open ? '▾' : '▴'}
		</button>
		{#if tool.kind !== null}
			<span
				class="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
				style:background-color={css(SEGMENT_ROLE_COLORS[tool.kind] ?? [0.5, 0.5, 0.5])}
				aria-hidden="true"
			></span>
		{/if}
		<!-- Insécables : coordonnées, vitesse et temps ne se coupent jamais en
		     deux lignes (seul le type de ligne, en dernier, s'abrège). -->
		<span class="whitespace-nowrap text-content tabular-nums">
			X {num(tool.position[0], 3)}&nbsp;&nbsp;Y {num(tool.position[1], 3)}&nbsp;&nbsp;Z {num(
				tool.position[2],
				3
			)}
		</span>
		<span class="whitespace-nowrap tabular-nums">
			{$t('Speed: {value}', { value: num(tool.speed, 0, 'mm/s') })}
		</span>
		{#if time}<span
				class="whitespace-nowrap tabular-nums"
				data-testid="preview-tool-time"
				data-time={time.time.toFixed(3)}
			>
				{$t('Time: {value}', { value: timeText })}
			</span>{/if}
		{#if detail && coloration !== 'speed'}<span class="min-w-0 truncate">{detail}</span>{/if}
	</div>
</div>
