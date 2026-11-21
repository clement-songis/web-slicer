<script lang="ts">
	// Dialogue « Preferences » (Orca PreferencesDialog.cpp), regroupé en quatre
	// onglets : Général, Caméra et souris, Comportement, Presets. Chaque réglage
	// s'applique immédiatement et est mémorisé pour l'utilisateur connecté.
	import { THEME_PREFS, themePref, setTheme, type ThemePref } from '$lib/theme';
	import { LOCALES, languageName, locale, setLocale, type Locale } from '$lib/i18n';
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
		{ id: 'general', label: 'Général' },
		{ id: 'camera', label: 'Caméra et souris' },
		{ id: 'behaviour', label: 'Comportement' },
		{ id: 'presets', label: 'Presets' },
		// Orca Preferences › Graphismes, réduit à ce qui a un sens dans un navigateur.
		{ id: 'graphics', label: 'Graphismes' }
	];
	const FPS_CAPS = [0, 30, 60, 120];
	let tab = $state<TabId>('general');

	const themeLabels: Record<ThemePref, string> = {
		light: 'Clair',
		dark: 'Sombre',
		system: 'Système'
	};
	const DRAG_LABELS: Record<DragAction, string> = {
		rotate: 'Rotation',
		pan: 'Panoramique',
		none: 'Rien'
	};
	const BUTTONS: { key: 'leftDrag' | 'middleDrag' | 'rightDrag'; label: string }[] = [
		{ key: 'leftDrag', label: 'Bouton gauche' },
		{ key: 'middleDrag', label: 'Bouton du milieu' },
		{ key: 'rightDrag', label: 'Bouton droit' }
	];
	const FLUSH_LABELS: Record<AutoFlush, string> = {
		all: 'Couleur ou filament',
		color: 'Couleur seulement',
		disabled: 'Jamais'
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

	let cleared = $state<string | null>(null);
	function clearChoices() {
		const n = clearRememberedChoices();
		cleared = n ? `${n} choix effacé${n > 1 ? 's' : ''}.` : 'Aucun choix mémorisé.';
	}

	// Saisie libre (virgule acceptée), resynchronisée sur la valeur mémorisée.
	let orbitText = $derived(cam.orbitSpeed.toFixed(2).replace('.', ','));
	function commitOrbit(text: string) {
		const value = clampOrbitSpeed(Number(text.replace(',', '.')));
		updateCamera({ orbitSpeed: value });
		orbitText = value.toFixed(2).replace('.', ',');
	}
</script>

{#snippet row(label: string, hint: string | null, control: import('svelte').Snippet, id: string)}
	<div class="flex items-start justify-between gap-4 py-2" data-pref-row={id}>
		<div class="min-w-0">
			<div class="text-sm text-content" id="pref-{id}-label">{label}</div>
			{#if hint}<p class="mt-0.5 text-xs leading-5 text-content-subtle">{hint}</p>{/if}
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
			aria-label="Préférences"
			data-testid="preferences-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onclose()}
		>
			<div class="px-4 pt-4">
				<h2 class="ws-dialog-title mb-3">Préférences</h2>
				<div class="flex gap-1 border-b border-border" role="tablist" aria-label="Rubriques">
					{#each TABS as t (t.id)}
						<button
							type="button"
							role="tab"
							aria-selected={tab === t.id}
							data-testid="pref-tab-{t.id}"
							class="ws-tab text-sm {tab === t.id ? 'ws-tab-on' : ''}"
							onclick={() => (tab = t.id)}>{t.label}</button
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
								<option value={pref}>{themeLabels[pref]}</option>
							{/each}
						</select>
					{/snippet}
					{@render row('Thème', null, themeCtl, 'theme')}
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
					{@render row('Langue', null, langCtl, 'language')}
					{#snippet unitsCtl()}
						<select
							data-testid="pref-units"
							aria-labelledby="pref-units-label"
							class="ws-field ws-field-sm w-40"
							value={prefs.units}
							onchange={(e) => update({ units: e.currentTarget.value as Units })}
						>
							<option value="mm">Métrique (mm)</option>
							<option value="in">Impérial (pouces)</option>
						</select>
					{/snippet}
					{@render row('Unités', null, unitsCtl, 'units')}
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
						'Devise des coûts',
						'Unité du « Coût du filament » (par kg) saisi dans les réglages du filament.',
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
						'Avertir avant de fermer',
						'Demande confirmation en quittant un projet non sauvegardé.',
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
							<option value="home">Accueil (projets)</option>
							<option value="prepare">Dernier projet</option>
						</select>
					{/snippet}
					{@render row(
						'Page au démarrage',
						'Page ouverte après la connexion ou à l’adresse du site : la liste des projets, ou directement le dernier projet modifié.',
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
						'Projets récents affichés',
						'Nombre de projets de « Récemment ouvert » sur la page d’accueil (0 à 999) ; les autres restent accessibles par la recherche ou « Tout afficher ».',
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
							<option value="load_all">Tout charger</option>
							<option value="ask_when_relevant">Demander si utile</option>
							<option value="always_ask">Toujours demander</option>
							<option value="load_geometry_only">Géométrie seule</option>
						</select>
					{/snippet}
					{@render row(
						'Ouverture d’un 3MF',
						'Faut-il charger les réglages (imprimante, filament, processus) à l’ouverture d’un fichier 3MF ? « Demander si utile » : la question n’est posée que si un projet est déjà ouvert (préférence « Load behaviour » d’Orca).',
						loadCtl,
						'load-behaviour'
					)}
				{:else if tab === 'camera'}
					{#snippet styleCtl()}
						<div class="ws-seg" role="radiogroup" aria-labelledby="pref-camera-style-label">
							{#each [['default', 'Souris'], ['touchpad', 'Pavé tactile']] as [value, label] (value)}
								<button
									type="button"
									role="radio"
									aria-checked={cam.style === value}
									data-testid="pref-camera-style-{value}"
									class="ws-seg-item {cam.style === value ? 'ws-seg-on' : ''}"
									onclick={() => updateCamera({ style: value as CameraPreferences['style'] })}
									>{label}</button
								>
							{/each}
						</div>
					{/snippet}
					{@render row(
						'Style de navigation',
						touchpad
							? 'Alt + déplacement : rotation · Maj + déplacement : panoramique · défilement : zoom.'
							: 'Glisser avec les boutons de la souris (actions ci-dessous) ; molette : zoom.',
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
										<option {value}>{label}</option>
									{/each}
								</select>
							{/snippet}
							{@render row(b.label + ' (glisser)', null, dragCtl, b.key)}
						{/each}
						{#if noRotate}
							<p
								class="rounded border border-warning/40 bg-warning-soft px-2 py-1 text-xs text-warning-text"
								data-testid="pref-no-rotate"
								role="status"
							>
								Aucun bouton ne fait tourner la vue : utilisez les vues prédéfinies (menu Vue) pour
								changer d’angle.
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
					{@render row(
						'Vitesse de rotation',
						'Multiplicateur de 0,05 à 2 (1 = normal).',
						orbitCtl,
						'orbit'
					)}
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
						'Zoomer vers le pointeur',
						'Le zoom se rapproche du point visé plutôt que du centre de la vue.',
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
					{@render row('Inverser le sens de la molette', null, reverseCtl, 'reverse-wheel')}
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
						'Caméra libre',
						'Rotation sans contrainte dans toutes les directions. Désactivée : l’axe Z reste vertical (recommandé).',
						freeCtl,
						'free-camera'
					)}
					<div
						class="flex items-center justify-between gap-2 border-t border-border pt-2 text-xs text-content-subtle"
					>
						<span>Souris 3D (3Dconnexion) : non prise en charge par le navigateur.</span>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="pref-camera-reset"
							disabled={cameraIsDefault}
							onclick={() => update({ camera: { ...CAMERA_DEFAULTS } })}>Valeurs par défaut</button
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
						'Avertir du G-code modifié',
						'À l’ouverture d’un 3MF dont les presets imprimante ou filament contiennent du G-code modifié (démarrage, fin, changement de filament…), une boîte liste ces G-code pour que vous vérifiiez qu’ils sont sans danger pour la machine. Décoché : le 3MF s’ouvre sans avertissement.',
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
						'Arranger automatiquement',
						'Réorganise le plateau après un clonage ou l’ajout d’un modèle pratique, pour éviter les chevauchements.',
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
						'Recalculer les volumes de purge',
						'Après un changement de filament, les purges vers et depuis ce filament sont recalculées ; les autres valeurs saisies sont conservées.',
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
						'Détecter les modèles en pouces ou en mètres',
						'À l’import, un objet minuscule (moins de 8 mm³) propose une conversion en millimètres.',
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
						'Garder la peinture après Simplifier / Réparer',
						'Reporte supports, couture, couleurs et surface floue peints sur le nouveau maillage. Désactivé : la peinture est effacée, comme dans OrcaSlicer.',
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
						'Autoriser les filaments haute et basse température ensemble',
						'Par défaut, un mélange comme PLA + PC est refusé au tranchage (risque de bouchage). Activé : simple avertissement.',
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
						'Choisir la répartition des filaments avant de trancher',
						'Imprimantes à deux buses (Bambu Lab H2D) : une fenêtre propose, à chaque tranchage, la buse de chaque filament (automatique ou manuelle).',
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
						'Options de maillage à l’import STEP',
						'Affiche le réglage de précision après chaque import STEP ; sinon les derniers réglages s’appliquent.',
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
							<span class="text-xs text-content-subtle">bits</span>
						</span>
					{/snippet}
					{@render row(
						'Niveau de qualité pour l’exportation Draco',
						'Quantification du maillage à l’export DRC. 0 = sans perte (géométrie exacte) ; de 8 à 30 : fichier plus petit, détails réduits quand la valeur baisse.',
						dracoCtl,
						'draco-bits'
					)}
					{#snippet clearCtl()}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-secondary"
							data-testid="pref-clear-choices"
							onclick={clearChoices}>Effacer</button
						>
					{/snippet}
					{@render row(
						'Choix « Ne plus demander »',
						cleared ?? REMEMBERED_CHOICES.map((c) => c.label).join(' · '),
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
						'Anticrénelage (MSAA)',
						'Lisse les arêtes des modèles et des parcours. Désactivé : rendu plus rapide sur un ordinateur ou une carte graphique modeste. La vue 3D est recréée.',
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
								<option value={cap}>{cap === 0 ? 'Illimité (écran)' : `${cap} images/s`}</option>
							{/each}
						</select>
					{/snippet}
					{@render row(
						'Images par seconde maximum',
						'Plafonne le rafraîchissement de la vue 3D pendant les mouvements (Orca « FPS max ») : moins de chauffe et de batterie consommée sur un portable.',
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
						'Afficher le compteur d’images/s',
						'Compteur en coin de la vue 3D. La vue ne se redessine que lorsqu’elle change : 0 au repos est normal.',
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
						'Afficher le navigateur 3D',
						'Cube d’orientation en bas à gauche de la vue de préparation : cliquez une face pour regarder le plateau de ce côté (aussi dans le menu Vue).',
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
						'Afficher les axes',
						'Repère X (rouge), Y (vert), Z (bleu) à l’origine du plateau.',
						axesCtl,
						'show-axes'
					)}
					<!-- Phase 38 — Orca Preferences › Graphics › Realistic View. -->
					<h3 class="ws-section-title mt-3">Vue réaliste</h3>
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
						'Activer la vue réaliste',
						'Ombrage plus fin des modèles en préparation (aussi dans le menu Vue). Suspendue pendant la peinture des supports, de la couture et du fuzzy skin pour bien voir la peinture.',
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
						'Ombrage Phong',
						'Éclairage calculé par pixel, avec reflets. Décoché : ombrage mat sans reflets (« Gouraud » d’Orca), plus léger.',
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
						'Occlusion ambiante SSAO',
						'Assombrit légèrement les creux et les recoins pour mieux lire le relief (une passe d’écran en plus, comme Orca).',
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
						'Ombres sur le plateau',
						'Les objets projettent leur ombre sur le plateau (lumière qui suit la caméra).',
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
						'Lisser les surfaces courbes',
						'Adoucit les facettes des surfaces arrondies ; les arêtes vives (plus de 5°) restent nettes.',
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
								<option value={g.value}>{g.label}</option>
							{/each}
						</select>
					{/snippet}
					{@render row(
						'Grouper mes presets filament',
						'Rangement de vos presets filament dans la liste des filaments : un seul groupe « Mes presets », sans sous-groupe, par type (PLA, PETG…) ou par fabricant.',
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
						'Afficher les presets incompatibles',
						'Les filaments et traitements prévus pour d’autres imprimantes apparaissent grisés dans les listes, sans pouvoir être choisis.',
						unsupportedCtl,
						'show-unsupported'
					)}
				{/if}
			</div>

			<div class="flex items-center justify-between gap-3 border-t border-border px-4 py-3">
				<p class="text-xs text-content-subtle">Enregistré automatiquement pour votre compte.</p>
				<button
					type="button"
					class="ws-btn ws-btn-primary"
					data-testid="preferences-close"
					onclick={onclose}>Fermer</button
				>
			</div>
		</div>
	</div>
{/if}
