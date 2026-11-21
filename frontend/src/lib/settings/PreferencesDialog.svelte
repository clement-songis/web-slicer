<script lang="ts">
	// Dialogue « Preferences » (Orca PreferencesDialog.cpp), regroupé en quatre
	// onglets : Général, Caméra et souris, Comportement, Presets. Chaque réglage
	// s'applique immédiatement et est mémorisé pour l'utilisateur connecté.
	import { THEME_PREFS, themePref, setTheme, type ThemePref } from '$lib/theme';
	import { LOCALES, fmtNum, languageName, locale, setLocale, t, type Locale } from '$lib/i18n';
	import {
		CAMERA_DEFAULTS,
		REMEMBERED_CHOICES,
		clampFpsCap,
		clampDracoBits,
		clampOrbitSpeed,
		clampRecentCount,
		clearRememberedChoices,
		savePreferences,
		type AppPreferences,
		type AutoFlush,
		type CameraPreferences,
		type DefaultPage,
		type DragAction,
		type GraphicsPreferences,
		type ProjectLoadBehaviour,
		type Units
	} from './preferences';
	import { USER_GROUPINGS, type UserGrouping } from '$lib/presets/user-grouping';
	import { loadStepPrefs, saveStepPrefs } from '$lib/editor/step-mesh';
	import { dialogFocus } from '$lib/ui/dialog-focus';

	interface Props {
		open: boolean;
		prefs: AppPreferences;
		onchange: (prefs: AppPreferences) => void;
		onclose: () => void;
	}

	let { open, prefs, onchange, onclose }: Props = $props();

	type TabId = 'general' | 'camera' | 'behaviour' | 'presets' | 'graphics';
	const TABS: { id: TabId; label: string }[] = [
		{ id: 'general', label: 'General' },
		{ id: 'camera', label: 'Camera & Mouse' },
		{ id: 'behaviour', label: 'Behaviour' },
		{ id: 'presets', label: 'Presets' },
		// Orca Preferences › Graphismes, réduit à ce qui a un sens dans un navigateur.
		{ id: 'graphics', label: 'Graphics' }
	];
	const FPS_CAPS = [0, 30, 60, 120];
	let tab = $state<TabId>('general');

	const themeLabels: Record<ThemePref, string> = {
		light: 'Light',
		dark: 'Dark',
		system: 'System'
	};
	const DRAG_LABELS: Record<DragAction, string> = {
		rotate: 'Rotation',
		pan: 'Pan',
		none: 'None||drag action'
	};
	const BUTTONS: { key: 'leftDrag' | 'middleDrag' | 'rightDrag'; label: string }[] = [
		{ key: 'leftDrag', label: 'Left button' },
		{ key: 'middleDrag', label: 'Middle button' },
		{ key: 'rightDrag', label: 'Right button' }
	];
	const FLUSH_LABELS: Record<AutoFlush, string> = {
		all: 'Color or filament',
		color: 'Color only',
		disabled: 'Never'
	};

	function update(patch: Partial<AppPreferences>) {
		const next = { ...prefs, ...patch };
		savePreferences(next);
		onchange(next);
	}
	function updateCamera(patch: Partial<CameraPreferences>) {
		update({ camera: { ...prefs.camera, ...patch } });
	}
	function updateGraphics(patch: Partial<GraphicsPreferences>) {
		update({ graphics: { ...prefs.graphics, ...patch } });
	}

	const cam = $derived(prefs.camera);
	const touchpad = $derived(cam.style === 'touchpad');
	const cameraIsDefault = $derived(
		(Object.keys(CAMERA_DEFAULTS) as (keyof CameraPreferences)[]).every(
			(k) => cam[k] === CAMERA_DEFAULTS[k]
		)
	);
	const noRotate = $derived(!touchpad && !BUTTONS.some((b) => cam[b.key] === 'rotate'));

	// Dialogue STEP (Orca enable_step_mesh_setting) : préférence du module STEP.
	let stepAsk = $state(true);
	$effect(() => {
		if (open) stepAsk = loadStepPrefs().ask;
	});
	function setStepAsk(ask: boolean) {
		stepAsk = ask;
		saveStepPrefs({ ...loadStepPrefs(), ask });
	}

	let cleared = $state<number | null>(null);
	function clearChoices() {
		cleared = clearRememberedChoices();
	}
	const clearedText = $derived(
		cleared === null
			? null
			: cleared
				? $t(cleared > 1 ? '{n} choices cleared.' : '{n} choice cleared.', { n: cleared })
				: $t('No remembered choice.')
	);

	// Saisie libre (virgule acceptée), resynchronisée sur la valeur mémorisée.
	let orbitText = $derived(fmtNum(cam.orbitSpeed, 2, $locale));
	function commitOrbit(text: string) {
		const value = clampOrbitSpeed(Number(text.replace(',', '.')));
		updateCamera({ orbitSpeed: value });
		orbitText = fmtNum(value, 2, $locale);
	}
</script>

{#snippet row(label: string, hint: string | null, control: import('svelte').Snippet, id: string)}
	<div class="flex items-start justify-between gap-4 py-2" data-pref-row={id}>
		<div class="min-w-0">
			<div class="text-sm text-content" id="pref-{id}-label">{$t(label)}</div>
			{#if hint}<p class="mt-0.5 text-xs leading-5 text-content-subtle">{$t(hint)}</p>{/if}
		</div>
		<div class="shrink-0">{@render control()}</div>
	</div>
{/snippet}

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim"
		role="presentation"
		onclick={onclose}
	>
		<div
			class="ws-dialog flex max-h-[calc(100vh-2rem)] w-[38rem] max-w-[calc(100vw-2rem)] flex-col p-0"
			role="dialog"
			aria-modal="true"
			aria-label={$t('Preferences')}
			data-testid="preferences-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onclose()}
		>
			<div class="px-4 pt-4">
				<h2 class="ws-dialog-title mb-3">{$t('Preferences')}</h2>
				<div class="flex gap-1 border-b border-border" role="tablist" aria-label={$t('Sections')}>
					{#each TABS as tb (tb.id)}
						<button
							type="button"
							role="tab"
							aria-selected={tab === tb.id}
							data-testid="pref-tab-{tb.id}"
							class="ws-tab text-sm {tab === tb.id ? 'ws-tab-on' : ''}"
							onclick={() => (tab = tb.id)}>{$t(tb.label)}</button
						>
					{/each}
				</div>
			</div>

			<div class="min-h-[22rem] flex-1 overflow-y-auto px-4 py-2" role="tabpanel">
				{#if tab === 'general'}
					{#snippet themeCtl()}
						<select
							data-testid="pref-theme"
							aria-labelledby="pref-theme-label"
							class="ws-field ws-field-sm w-40"
							value={$themePref}
							onchange={(e) => setTheme(e.currentTarget.value as ThemePref)}
						>
							{#each THEME_PREFS as pref (pref)}
								<option value={pref}>{$t(themeLabels[pref])}</option>
							{/each}
						</select>
					{/snippet}
					{@render row('Theme', null, themeCtl, 'theme')}
					{#snippet langCtl()}
						<select
							data-testid="pref-language"
							aria-labelledby="pref-language-label"
							class="ws-field ws-field-sm w-40"
							value={$locale}
							onchange={(e) => setLocale(e.currentTarget.value as Locale)}
						>
							{#each LOCALES as code (code)}
								<option value={code}>{languageName(code)}</option>
							{/each}
						</select>
					{/snippet}
					{@render row('Language', null, langCtl, 'language')}
					{#snippet unitsCtl()}
						<select
							data-testid="pref-units"
							aria-labelledby="pref-units-label"
							class="ws-field ws-field-sm w-40"
							value={prefs.units}
							onchange={(e) => update({ units: e.currentTarget.value as Units })}
						>
							<option value="mm">{$t('Metric (mm)')}</option>
							<option value="in">{$t('Imperial (inches)')}</option>
						</select>
					{/snippet}
					{@render row('Units', null, unitsCtl, 'units')}
					{#snippet currencyCtl()}
						<input
							data-testid="pref-currency"
							aria-labelledby="pref-currency-label"
							class="ws-field ws-field-sm w-20 text-center"
							maxlength="4"
							value={prefs.currency}
							onchange={(e) => update({ currency: e.currentTarget.value.trim() || '€' })}
						/>
					{/snippet}
					{@render row(
						'Cost currency',
						'Unit of the “Filament cost” (per kg) entered in the filament settings.',
						currencyCtl,
						'currency'
					)}
					{#snippet closeCtl()}
						<input
							type="checkbox"
							data-testid="pref-confirm-close"
							aria-labelledby="pref-confirm-close-label"
							checked={prefs.confirmClose}
							onchange={(e) => update({ confirmClose: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Warn before closing',
						'Asks for confirmation when leaving an unsaved project.',
						closeCtl,
						'confirm-close'
					)}
					{#snippet startCtl()}
						<select
							data-testid="pref-default-page"
							aria-labelledby="pref-default-page-label"
							class="ws-field ws-field-sm w-40"
							value={prefs.defaultPage}
							onchange={(e) => update({ defaultPage: e.currentTarget.value as DefaultPage })}
						>
							<option value="home">{$t('Home (projects)')}</option>
							<option value="prepare">{$t('Last project')}</option>
						</select>
					{/snippet}
					{@render row(
						'Startup page',
						'Page opened after signing in or at the site address: the project list, or directly the last modified project.',
						startCtl,
						'default-page'
					)}
					{#snippet recentCtl()}
						<input
							type="number"
							min="0"
							max="999"
							step="1"
							data-testid="pref-max-recent"
							aria-labelledby="pref-max-recent-label"
							class="ws-field ws-field-sm w-20 text-center"
							value={prefs.maxRecent}
							onchange={(e) => {
								const v = clampRecentCount(Number(e.currentTarget.value));
								e.currentTarget.value = String(v);
								update({ maxRecent: v });
							}}
						/>
					{/snippet}
					{@render row(
						'Recent projects shown',
						'Number of projects in “Recently opened” on the home page (0 to 999); the others remain reachable through search or “Show all”.',
						recentCtl,
						'max-recent'
					)}
					{#snippet loadCtl()}
						<select
							data-testid="pref-load-behaviour"
							aria-labelledby="pref-load-behaviour-label"
							class="ws-field ws-field-sm w-52"
							value={prefs.projectLoadBehaviour}
							onchange={(e) =>
								update({ projectLoadBehaviour: e.currentTarget.value as ProjectLoadBehaviour })}
						>
							<option value="load_all">{$t('Load All')}</option>
							<option value="ask_when_relevant">{$t('Ask When Relevant')}</option>
							<option value="always_ask">{$t('Always Ask')}</option>
							<option value="load_geometry_only">{$t('Load Geometry Only')}</option>
						</select>
					{/snippet}
					{@render row(
						'Opening a 3MF',
						'Should the settings (printer, filament, process) be loaded when opening a 3MF file? “Ask when relevant”: the question is only asked if a project is already open (Orca “Load behaviour” preference).',
						loadCtl,
						'load-behaviour'
					)}
				{:else if tab === 'camera'}
					{#snippet styleCtl()}
						<div class="ws-seg" role="radiogroup" aria-labelledby="pref-camera-style-label">
							{#each [['default', 'Mouse'], ['touchpad', 'Touchpad']] as [value, label] (value)}
								<button
									type="button"
									role="radio"
									aria-checked={cam.style === value}
									data-testid="pref-camera-style-{value}"
									class="ws-seg-item {cam.style === value ? 'ws-seg-on' : ''}"
									onclick={() => updateCamera({ style: value as CameraPreferences['style'] })}
									>{$t(label)}</button
								>
							{/each}
						</div>
					{/snippet}
					{@render row(
						'Navigation style',
						touchpad
							? 'Alt + move: rotate · Shift + move: pan · scroll: zoom.'
							: 'Drag with the mouse buttons (actions below); wheel: zoom.',
						styleCtl,
						'camera-style'
					)}
					{#if !touchpad}
						{#each BUTTONS as b (b.key)}
							{#snippet dragCtl()}
								<select
									data-testid="pref-{b.key}"
									aria-labelledby="pref-{b.key}-label"
									class="ws-field ws-field-sm w-40"
									value={cam[b.key]}
									onchange={(e) => updateCamera({ [b.key]: e.currentTarget.value as DragAction })}
								>
									{#each Object.entries(DRAG_LABELS) as [value, label] (value)}
										<option {value}>{$t(label)}</option>
									{/each}
								</select>
							{/snippet}
							{@render row($t('{button} (drag)', { button: $t(b.label) }), null, dragCtl, b.key)}
						{/each}
						{#if noRotate}
							<p
								class="rounded border border-warning/40 bg-warning-soft px-2 py-1 text-xs text-warning-text"
								data-testid="pref-no-rotate"
								role="status"
							>
								{$t(
									'No button rotates the view: use the preset views (View menu) to change the angle.'
								)}
							</p>
						{/if}
					{/if}
					{#snippet orbitCtl()}
						<div class="flex items-center gap-2">
							<input
								type="range"
								min="0.05"
								max="2"
								step="0.05"
								class="w-28"
								aria-labelledby="pref-orbit-label"
								data-testid="pref-orbit-range"
								value={cam.orbitSpeed}
								oninput={(e) =>
									updateCamera({ orbitSpeed: clampOrbitSpeed(Number(e.currentTarget.value)) })}
							/>
							<input
								class="ws-field ws-field-sm w-16 text-center"
								inputmode="decimal"
								aria-labelledby="pref-orbit-label"
								data-testid="pref-orbit"
								bind:value={orbitText}
								onchange={(e) => commitOrbit(e.currentTarget.value)}
								onkeydown={(e) => e.key === 'Enter' && commitOrbit(e.currentTarget.value)}
							/>
							<span class="text-xs text-content-subtle">×</span>
						</div>
					{/snippet}
					{@render row('Orbit speed', 'Multiplier from 0.05 to 2 (1 = normal).', orbitCtl, 'orbit')}
					{#snippet zoomCtl()}
						<input
							type="checkbox"
							data-testid="pref-zoom-to-mouse"
							aria-labelledby="pref-zoom-to-mouse-label"
							checked={cam.zoomToMouse}
							onchange={(e) => updateCamera({ zoomToMouse: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Zoom to mouse position',
						'Zooming moves toward the point under the cursor instead of the view center.',
						zoomCtl,
						'zoom-to-mouse'
					)}
					{#snippet reverseCtl()}
						<input
							type="checkbox"
							data-testid="pref-reverse-wheel"
							aria-labelledby="pref-reverse-wheel-label"
							checked={cam.reverseWheel}
							onchange={(e) => updateCamera({ reverseWheel: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row('Reverse mouse wheel', null, reverseCtl, 'reverse-wheel')}
					{#snippet freeCtl()}
						<input
							type="checkbox"
							data-testid="pref-free-camera"
							aria-labelledby="pref-free-camera-label"
							checked={cam.freeCamera}
							onchange={(e) => updateCamera({ freeCamera: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Free camera',
						'Unconstrained rotation in all directions. Off: the Z axis stays vertical (recommended).',
						freeCtl,
						'free-camera'
					)}
					<div
						class="flex items-center justify-between gap-2 border-t border-border pt-2 text-xs text-content-subtle"
					>
						<span>{$t('3D mouse (3Dconnexion): not supported by the browser.')}</span>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="pref-camera-reset"
							disabled={cameraIsDefault}
							onclick={() => update({ camera: { ...CAMERA_DEFAULTS } })}>{$t('Defaults')}</button
						>
					</div>
				{:else if tab === 'behaviour'}
					{#snippet gcodeWarnCtl()}
						<input
							type="checkbox"
							data-testid="pref-gcode-warn"
							aria-labelledby="pref-gcode-warn-label"
							checked={!prefs.noWarnWhenModifiedGcodes}
							onchange={(e) => update({ noWarnWhenModifiedGcodes: !e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Warn about modified G-code',
						'When opening a 3MF whose printer or filament presets contain modified G-code (start, end, filament change…), a dialog lists those G-codes so you can check they are safe for the machine. Unchecked: the 3MF opens without warning.',
						gcodeWarnCtl,
						'gcode-warn'
					)}
					{#snippet arrangeCtl()}
						<input
							type="checkbox"
							data-testid="pref-auto-arrange"
							aria-labelledby="pref-auto-arrange-label"
							checked={prefs.autoArrange}
							onchange={(e) => update({ autoArrange: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Auto arrange',
						'Rearranges the plate after cloning or adding a handy model, to avoid overlaps.',
						arrangeCtl,
						'auto-arrange'
					)}
					{#snippet flushCtl()}
						<select
							data-testid="pref-auto-flush"
							aria-labelledby="pref-auto-flush-label"
							class="ws-field ws-field-sm w-44"
							value={prefs.autoFlush}
							onchange={(e) => update({ autoFlush: e.currentTarget.value as AutoFlush })}
						>
							{#each Object.entries(FLUSH_LABELS) as [value, label] (value)}
								<option {value}>{label}</option>
							{/each}
						</select>
					{/snippet}
					{@render row(
						'Auto-calculate flushing volumes',
						'After a filament change, flushes to and from that filament are recalculated; other entered values are kept.',
						flushCtl,
						'auto-flush'
					)}
					{#snippet unitCtl()}
						<input
							type="checkbox"
							data-testid="pref-unit-conversion"
							aria-labelledby="pref-unit-conversion-label"
							checked={prefs.unitConversionPrompt}
							onchange={(e) => update({ unitConversionPrompt: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Detect models in inches or meters',
						'On import, a tiny object (under 8 mm³) offers a conversion to millimeters.',
						unitCtl,
						'unit-conversion'
					)}
					{#snippet keepPaintCtl()}
						<input
							type="checkbox"
							data-testid="pref-keep-painting"
							aria-labelledby="pref-keep-painting-label"
							checked={prefs.keepPainting}
							onchange={(e) => update({ keepPainting: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Keep painting after Simplify / Repair',
						'Transfers painted supports, seam, colors and fuzzy skin onto the new mesh. Off: painting is cleared, as in OrcaSlicer.',
						keepPaintCtl,
						'keep-painting'
					)}
					{#snippet mixedCtl()}
						<input
							type="checkbox"
							data-testid="pref-mixed-temperature"
							aria-labelledby="pref-mixed-temperature-label"
							checked={prefs.mixedTemperature}
							onchange={(e) => update({ mixedTemperature: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Allow high and low temperature filaments together',
						'By default, a mix like PLA + PC is refused at slicing (clogging risk). On: a warning only.',
						mixedCtl,
						'mixed-temperature'
					)}
					{#snippet fmapCtl()}
						<input
							type="checkbox"
							data-testid="pref-filament-map-popup"
							aria-labelledby="pref-filament-map-popup-label"
							checked={prefs.filamentMapPopup}
							onchange={(e) => update({ filamentMapPopup: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Choose filament grouping before slicing',
						'Dual-nozzle printers (Bambu Lab H2D): a dialog offers, at each slice, the nozzle for each filament (automatic or manual).',
						fmapCtl,
						'filament-map-popup'
					)}
					{#snippet stepCtl()}
						<input
							type="checkbox"
							data-testid="pref-step-dialog"
							aria-labelledby="pref-step-dialog-label"
							checked={stepAsk}
							onchange={(e) => setStepAsk(e.currentTarget.checked)}
						/>
					{/snippet}
					{@render row(
						'Mesh options on STEP import',
						'Shows the precision setting after each STEP import; otherwise the last settings apply.',
						stepCtl,
						'step-dialog'
					)}
					{#snippet dracoCtl()}
						<span class="flex items-center gap-1">
							<input
								type="number"
								min="0"
								max="30"
								step="1"
								class="ws-field ws-field-sm w-20 text-right tabular-nums"
								data-testid="pref-draco-bits"
								aria-labelledby="pref-draco-bits-label"
								value={prefs.dracoBits}
								onchange={(e) => {
									const bits = clampDracoBits(Number(e.currentTarget.value));
									e.currentTarget.value = String(bits);
									update({ dracoBits: bits });
								}}
							/>
							<span class="text-xs text-content-subtle">{$t('bits')}</span>
						</span>
					{/snippet}
					{@render row(
						'Draco export quality level',
						'Mesh quantization for DRC export. 0 = lossless (exact geometry); 8 to 30: smaller file, fewer details as the value decreases.',
						dracoCtl,
						'draco-bits'
					)}
					{#snippet clearCtl()}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-secondary"
							data-testid="pref-clear-choices"
							onclick={clearChoices}>{$t('Clear')}</button
						>
					{/snippet}
					{@render row(
						'“Don’t ask again” choices',
						clearedText ?? REMEMBERED_CHOICES.map((c) => $t(c.label)).join(' · '),
						clearCtl,
						'clear-choices'
					)}
				{:else if tab === 'graphics'}
					{#snippet aaCtl()}
						<input
							type="checkbox"
							data-testid="pref-antialias"
							aria-labelledby="pref-antialias-label"
							checked={prefs.graphics.antialias}
							onchange={(e) => updateGraphics({ antialias: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Anti-aliasing (MSAA)',
						'Smooths the edges of models and toolpaths. Off: faster rendering on a modest computer or graphics card. The 3D view is recreated.',
						aaCtl,
						'antialias'
					)}
					{#snippet fpsCapCtl()}
						<select
							data-testid="pref-fps-cap"
							aria-labelledby="pref-fps-cap-label"
							class="ws-field ws-field-sm w-44"
							value={prefs.graphics.fpsCap}
							onchange={(e) =>
								updateGraphics({ fpsCap: clampFpsCap(Number(e.currentTarget.value)) })}
						>
							{#each FPS_CAPS as cap (cap)}
								<option value={cap}
									>{cap === 0 ? $t('Unlimited (display)') : $t('{cap} FPS', { cap })}</option
								>
							{/each}
						</select>
					{/snippet}
					{@render row(
						'Maximum frames per second',
						'Caps the 3D view refresh rate during movement (Orca “Max FPS”): less heat and battery drain on a laptop.',
						fpsCapCtl,
						'fps-cap'
					)}
					{#snippet fpsCtl()}
						<input
							type="checkbox"
							data-testid="pref-show-fps"
							aria-labelledby="pref-show-fps-label"
							checked={prefs.graphics.showFps}
							onchange={(e) => updateGraphics({ showFps: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Show FPS counter',
						'Counter in the corner of the 3D view. The view only redraws when it changes: 0 at rest is normal.',
						fpsCtl,
						'show-fps'
					)}
					{#snippet navCtl()}
						<input
							type="checkbox"
							data-testid="pref-3d-navigator"
							aria-labelledby="pref-3d-navigator-label"
							checked={prefs.show3dNavigator}
							onchange={(e) => update({ show3dNavigator: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Show 3D Navigator',
						'Orientation cube at the bottom left of the Prepare view: click a face to look at the plate from that side (also in the View menu).',
						navCtl,
						'3d-navigator'
					)}
					{#snippet axesCtl()}
						<input
							type="checkbox"
							data-testid="pref-show-axes"
							aria-labelledby="pref-show-axes-label"
							checked={prefs.showAxes}
							onchange={(e) => update({ showAxes: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Show axes',
						'X (red), Y (green), Z (blue) axes at the plate origin.',
						axesCtl,
						'show-axes'
					)}
					<!-- Phase 38 — Orca Preferences › Graphics › Realistic View. -->
					<h3 class="ws-section-title mt-3">{$t('Realistic View')}</h3>
					{#snippet realisticCtl()}
						<input
							type="checkbox"
							data-testid="pref-realistic-view"
							aria-labelledby="pref-realistic-view-label"
							checked={prefs.graphics.realisticView}
							onchange={(e) => updateGraphics({ realisticView: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Enable realistic view',
						'Finer shading of models in Prepare (also in the View menu). Suspended while painting supports, seam and fuzzy skin so the painting stays visible.',
						realisticCtl,
						'realistic-view'
					)}
					{#snippet phongCtl()}
						<input
							type="checkbox"
							data-testid="pref-realistic-phong"
							aria-labelledby="pref-realistic-phong-label"
							disabled={!prefs.graphics.realisticView}
							checked={prefs.graphics.realisticPhong}
							onchange={(e) => updateGraphics({ realisticPhong: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Phong shading',
						'Per-pixel lighting with highlights. Unchecked: matte shading without highlights (Orca “Gouraud”), lighter.',
						phongCtl,
						'realistic-phong'
					)}
					{#snippet ssaoCtl()}
						<input
							type="checkbox"
							data-testid="pref-realistic-ssao"
							aria-labelledby="pref-realistic-ssao-label"
							disabled={!prefs.graphics.realisticView}
							checked={prefs.graphics.realisticSsao}
							onchange={(e) => updateGraphics({ realisticSsao: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'SSAO ambient occlusion',
						'Slightly darkens hollows and corners to read the relief better (one extra screen pass, like Orca).',
						ssaoCtl,
						'realistic-ssao'
					)}
					{#snippet shadowsCtl()}
						<input
							type="checkbox"
							data-testid="pref-realistic-shadows"
							aria-labelledby="pref-realistic-shadows-label"
							disabled={!prefs.graphics.realisticView}
							checked={prefs.graphics.realisticShadows}
							onchange={(e) => updateGraphics({ realisticShadows: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Shadows on the plate',
						'Objects cast their shadow on the plate (light follows the camera).',
						shadowsCtl,
						'realistic-shadows'
					)}
					{#snippet smoothCtl()}
						<input
							type="checkbox"
							data-testid="pref-realistic-smooth"
							aria-labelledby="pref-realistic-smooth-label"
							disabled={!prefs.graphics.realisticView}
							checked={prefs.graphics.realisticSmoothNormals}
							onchange={(e) => updateGraphics({ realisticSmoothNormals: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Smooth curved surfaces',
						'Softens the facets of rounded surfaces; sharp edges (over 5°) stay crisp.',
						smoothCtl,
						'realistic-smooth'
					)}
				{:else}
					{#snippet groupCtl()}
						<select
							data-testid="pref-group-filament-presets"
							aria-labelledby="pref-group-filament-presets-label"
							class="ws-field ws-field-sm w-44"
							value={prefs.groupFilamentPresets}
							onchange={(e) =>
								update({ groupFilamentPresets: e.currentTarget.value as UserGrouping })}
						>
							{#each USER_GROUPINGS as g (g.value)}
								<option value={g.value}>{$t(g.label)}</option>
							{/each}
						</select>
					{/snippet}
					{@render row(
						'Group my filament presets',
						'How your filament presets are arranged in the filament list: a single “My Presets” group, no subgroup, by type (PLA, PETG…) or by vendor.',
						groupCtl,
						'group-filament-presets'
					)}
					{#snippet unsupportedCtl()}
						<input
							type="checkbox"
							data-testid="pref-show-unsupported"
							aria-labelledby="pref-show-unsupported-label"
							checked={prefs.showUnsupportedPresets}
							onchange={(e) => update({ showUnsupportedPresets: e.currentTarget.checked })}
						/>
					{/snippet}
					{@render row(
						'Show incompatible presets',
						'Filaments and processes meant for other printers appear greyed out in the lists, without being selectable.',
						unsupportedCtl,
						'show-unsupported'
					)}
				{/if}
			</div>

			<div class="flex items-center justify-between gap-3 border-t border-border px-4 py-3">
				<p class="text-xs text-content-subtle">{$t('Saved automatically for your account.')}</p>
				<button
					type="button"
					class="ws-btn ws-btn-primary"
					data-testid="preferences-close"
					onclick={onclose}>{$t('Close')}</button
				>
			</div>
		</div>
	</div>
{/if}
