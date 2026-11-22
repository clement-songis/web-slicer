<script lang="ts">
	// Liste d'objets de la scène (T053) : arbre objets/pièces/groupes avec
	// sélection, verrouillage/masquage, extrudeur par nœud, duplication et
	// suppression. Purement présentationnel : les mutations passent par des
	// callbacks (le parent applique sur l'ObjectTree et historise).
	import type { ObjectTree, SceneNode } from './objects.svelte';
	// Icônes SVG (lucide) : les glyphes emoji 🔒/👁/⧉ s'affichaient en « ▯ » sans police emoji.
	import IconLock from '~icons/lucide/lock';
	import IconUnlock from '~icons/lucide/lock-open';
	import IconEye from '~icons/lucide/eye';
	import IconEyeOff from '~icons/lucide/eye-off';
	import IconCopy from '~icons/lucide/copy';
	import IconX from '~icons/lucide/x';
	import IconSettings from '~icons/lucide/sliders-horizontal';
	import IconLayers from '~icons/lucide/layers';
	import IconBox from '~icons/lucide/box';
	import IconHash from '~icons/lucide/hash';
	import IconIssueError from '~icons/lucide/circle-alert';
	import IconIssueWarning from '~icons/lucide/triangle-alert';
	import IconSearch from '~icons/lucide/search';
	import IconScissors from '~icons/lucide/scissors';
	import IconWrench from '~icons/lucide/wrench';
	import type { Plate } from './plates.svelte';
	import { searchObjects } from './object-search';
	import { instanceListLayout } from './instances';
	import { t } from '$lib/i18n';

	interface Props {
		tree: ObjectTree;
		selection: Set<string>;
		onselect: (id: string, additive: boolean) => void;
		ontogglelock: (id: string) => void;
		ontogglehide: (id: string) => void;
		onextruder: (id: string, extruder: number) => void;
		onduplicate: (id: string) => void;
		ondelete: (id: string) => void;
		ongroup: () => void;
		/** Clic droit sur un nœud → menu contextuel objet (T112), au curseur. */
		oncontext?: (id: string, x: number, y: number) => void;
		/** Nombre de slots filament / extrudeurs sélectionnables (pour le sélecteur). */
		extruderCount?: number;
		/** Couleur CSS par slot (index 0 ↔ extrudeur 1), pour la pastille du nœud. */
		extruderColors?: readonly string[];
		/** Objets du plateau actif ; un groupe reste visible s'il en contient un. */
		visibleIds?: ReadonlySet<string>;
		/** Affectation d'un objet à un autre plateau. */
		plates?: readonly Plate[];
		onassign?: (id: string, plateId: string) => void;
		/** Renommage en ligne (double-clic ou menu contextuel). */
		onrename?: (id: string, name: string) => void;
		/** Objet en cours de renommage (lié au parent pour le menu contextuel). */
		editingId?: string | null;
		/** Surcharges par nœud (Orca : sous-éléments « Réglages », « Plages de
		 *  hauteur », modificateurs) : compteurs affichés en badges. */
		overrides?: Readonly<Record<string, { settings?: number; ranges?: number; parts?: number }>>;
		/** Réordonne un objet racine : il prend la place de `targetId` (glisser-
		 *  déposer, Alt+↑/↓). L'ordre de la liste est l'ordre d'impression
		 *  séquentielle (Orca : ordre du modèle). */
		onreorder?: (id: string, targetId: string) => void;
		/** Impression « objet par objet » active : rang d'impression affiché. */
		sequential?: boolean;
		/** Étiquettes de rang sur la scène 3D (objet par objet), masquables. */
		showOrderOnScene?: boolean;
		/** Problème par objet (phase 29) : erreur / avertissement de validation ou
		 *  échec de conversion du modèle, avec son texte (pastille + infobulle). */
		issues?: Readonly<
			Record<string, { level: 'error' | 'warning'; text: string; inline?: boolean }>
		>;
		/** Noms des pièces par objet (recherche « plateau, objet et pièce » d'Orca). */
		partNames?: Readonly<Record<string, readonly string[]>>;
		/** Résultat de recherche sur un autre plateau : l'activer et y sélectionner l'objet. */
		onlocate?: (plateId: string, id: string) => void;
		/**
		 * Lot 37 — erreurs de maillage par objet (Orca : icône obj_warning de la
		 * liste d'objets, infobulle get_mesh_errors_info) ; clic → réparer.
		 */
		meshWarnings?: Readonly<Record<string, { tooltip: string; openEdges: number }>>;
		onrepair?: (id: string) => void;
		/** Lot 37 — pièces issues d'une coupe : nombre de pièces liées (cutId). */
		cutLinks?: Readonly<Record<string, number>>;
	}

	let {
		tree,
		selection,
		onselect,
		ontogglelock,
		ontogglehide,
		onextruder,
		onduplicate,
		ondelete,
		ongroup,
		oncontext,
		extruderCount = 1,
		extruderColors = [],
		visibleIds,
		plates = [],
		onassign,
		onrename,
		editingId = $bindable(null),
		overrides = {},
		onreorder,
		sequential = false,
		showOrderOnScene = $bindable(true),
		issues = {},
		partNames = {},
		onlocate,
		meshWarnings = {},
		onrepair,
		cutLinks = {}
	}: Props = $props();

	// Phase 34 : recherche (Orca Plater m_search_bar). Filtre l'arbre du plateau
	// actif ; les résultats des autres plateaux sont proposés en liens.
	let query = $state('');
	const search = $derived(searchObjects(tree.list(), query, plates, partNames));
	const elsewhere = $derived(
		search
			? search.hits.filter((hit) => {
					const node = tree.get(hit.id);
					return !!node && !inPlate(node) && !!hit.plateId;
				})
			: []
	);
	const shownHere = $derived(search ? tree.roots().filter((n) => isVisible(n)).length : 0);

	// Objets racines réordonnables, dans l'ordre (= ordre d'impression).
	const orderable = (node: SceneNode) => node.kind === 'object' && node.parentId === null;
	const printRank = $derived.by(() => {
		const rank: Record<string, number> = {};
		let n = 0;
		for (const node of tree.roots())
			if (orderable(node) && inPlate(node) && tree.isPrintable(node.id)) rank[node.id] = ++n;
		return rank;
	});
	let dragId = $state<string | null>(null);
	let dropId = $state<string | null>(null);

	function neighbour(id: string, delta: -1 | 1): string | null {
		const ids = tree
			.roots()
			.filter((n) => orderable(n) && inPlate(n))
			.map((n) => n.id);
		const at = ids.indexOf(id);
		return at < 0 ? null : (ids[at + delta] ?? null);
	}

	const extruders = $derived(
		Array.from({ length: Math.max(1, Math.trunc(extruderCount)) }, (_, i) => i + 1)
	);

	// Un extrudeur hors des slots existants (slot retiré, projet incohérent) est
	// affiché « auto », comme il sera persisté à l'enregistrement.
	function shownExtruder(node: SceneNode): number {
		const ex = node.extruder > 0 && node.extruder <= extruders.length ? node.extruder : 0;
		// Phase 29 : un objet racine imprime toujours avec un filament ; comme la
		// colonne « Fila. » d'Orca (pas de « default » pour un objet), on montre
		// le n° et la pastille du filament 1 au lieu d'un « auto » énigmatique.
		return ex === 0 && rootObject(node) ? 1 : ex;
	}
	const rootObject = (node: SceneNode) => node.kind === 'object' && node.parentId === null;

	/** Nœud du plateau actif (ou groupe contenant un objet du plateau). */
	function inPlate(node: SceneNode): boolean {
		if (!visibleIds || visibleIds.has(node.id)) return true;
		return tree.children(node.id).some(inPlate);
	}
	/** Affiché : du plateau actif et, en recherche, résultat, ancêtre ou descendant d'un résultat. */
	function isVisible(node: SceneNode): boolean {
		if (!inPlate(node)) return false;
		if (!search || search.shown.has(node.id)) return true;
		for (let p = node.parentId ? tree.get(node.parentId) : undefined; p;) {
			if (search.matched.has(p.id)) return true;
			p = p.parentId ? tree.get(p.parentId) : undefined;
		}
		return false;
	}
</script>

{#snippet row(node: SceneNode, depth: number)}
	{@const selected = selection.has(node.id)}
	{@const ov = overrides[node.id]}
	{@const nSettings = ov?.settings ?? Object.keys(node.settings ?? {}).length}
	{#if isVisible(node)}
		<li>
			<div
				class="flex min-h-8 flex-wrap items-center gap-x-1 rounded-md px-1.5 py-0.5 transition-colors {selected
					? 'bg-primary text-primary-content'
					: 'hover:bg-overlay'}"
				data-testid="object-row"
				data-printable={node.printable}
				data-match={search?.matched.has(node.id) ? 'true' : undefined}
				class:ws-search-hit={!selected && !!search?.matched.has(node.id)}
				style="padding-left: {depth * 1 + 0.25}rem"
				role="presentation"
				class:ws-drop-target={dropId === node.id && dragId !== node.id}
				draggable={!!onreorder && orderable(node) && editingId !== node.id}
				ondragstart={(e) => {
					if (!onreorder || !orderable(node)) return;
					dragId = node.id;
					e.dataTransfer?.setData('text/x-ws-object', node.id);
					if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
				}}
				ondragover={(e) => {
					if (!dragId || !orderable(node)) return;
					e.preventDefault();
					if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
					dropId = node.id;
				}}
				ondragleave={() => {
					if (dropId === node.id) dropId = null;
				}}
				ondrop={(e) => {
					if (!dragId || !orderable(node)) return;
					e.preventDefault();
					if (dragId !== node.id) onreorder?.(dragId, node.id);
					dragId = dropId = null;
				}}
				ondragend={() => (dragId = dropId = null)}
				oncontextmenu={(e) => {
					if (!oncontext) return;
					e.preventDefault();
					// Comme Orca, un clic droit dans une multi-sélection la conserve.
					if (!selection.has(node.id)) onselect(node.id, false);
					oncontext(node.id, e.clientX, e.clientY);
				}}
			>
				{#if editingId === node.id && onrename}
					<input
						class="ws-field ws-field-sm min-w-0 flex-1 border-accent"
						data-testid="object-rename-input"
						aria-label={$t('New name for {name}', { name: node.name })}
						value={node.name}
						{@attach (el: HTMLInputElement) => {
							el.focus();
							el.select();
						}}
						onkeydown={(e) => {
							e.stopPropagation();
							if (e.key === 'Enter') {
								onrename(node.id, e.currentTarget.value);
								editingId = null;
							} else if (e.key === 'Escape') editingId = null;
						}}
						onblur={(e) => {
							if (editingId === node.id) onrename(node.id, e.currentTarget.value);
							editingId = null;
						}}
					/>
				{:else}
					{#if sequential && printRank[node.id]}
						<span
							class="inline-flex h-4 min-w-4 shrink-0 items-center justify-center rounded-full px-1 text-2xs font-semibold tabular-nums {selected
								? 'bg-primary-content text-primary'
								: 'bg-surface-sunken text-content-muted ring-1 ring-border-strong'}"
							data-testid="object-print-order"
							data-tip={$t(
								printRank[node.id] === 1
									? 'Printed {n}st (by object) · drag or Alt+↑/↓ to change the order'
									: 'Printed #{n} (by object) · drag or Alt+↑/↓ to change the order',
								{ n: printRank[node.id] }
							)}
							aria-label={$t('Print order: {n}', { n: printRank[node.id] })}
							>{printRank[node.id]}</span
						>
					{/if}
					<button
						type="button"
						class="max-w-full min-w-0 flex-[1_1_auto] truncate text-left text-sm {tree.isHidden(
							node.id
						) || !tree.isPrintable(node.id)
							? 'opacity-40'
							: ''}"
						data-testid="object-name"
						title={node.name}
						onclick={(e) => onselect(node.id, e.shiftKey || e.ctrlKey || e.metaKey)}
						ondblclick={() => onrename && (editingId = node.id)}
						onkeydown={(e) => {
							// Groupe « Objects List » d'Orca : Espace renomme l'objet/la pièce
							// sélectionné(e) ; F2 (convention bureau) aussi. Un premier Espace
							// sur une ligne non sélectionnée la sélectionne (bouton natif).
							if (
								onrename &&
								!e.altKey &&
								!e.ctrlKey &&
								!e.metaKey &&
								(e.key === 'F2' || (e.key === ' ' && selection.has(node.id)))
							) {
								e.preventDefault();
								e.stopPropagation();
								editingId = node.id;
								return;
							}
							if (!onreorder || !orderable(node) || !e.altKey) return;
							if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
							const other = neighbour(node.id, e.key === 'ArrowUp' ? -1 : 1);
							e.preventDefault();
							e.stopPropagation();
							if (other) onreorder(node.id, other);
						}}
					>
						<!-- Puce décorative : hérite de la couleur de la ligne (lisible aussi sur la
							     ligne sélectionnée, fond primaire) au lieu d'un gris fixe à 2:1. -->
						{#if !(sequential && printRank[node.id])}<span class="opacity-60" aria-hidden="true"
								>{node.kind === 'group' ? '▾' : '•'}</span
							>{/if}
						{node.name}
					</button>
					{#if issues[node.id]}
						{@const issue = issues[node.id]}
						<span
							class="inline-flex shrink-0 items-center {selected
								? ''
								: issue.level === 'error'
									? 'text-danger'
									: 'text-warning'}"
							data-testid="object-issue"
							data-level={issue.level}
							data-tip={issue.text}
							role="img"
							aria-label={$t(issue.level === 'error' ? 'Error: {text}' : 'Warning: {text}', {
								text: issue.text
							})}
						>
							{#if issue.level === 'error'}<IconIssueError
									class="h-3.5 w-3.5"
									aria-hidden="true"
								/>{:else}<IconIssueWarning class="h-3.5 w-3.5" aria-hidden="true" />{/if}
						</span>
					{/if}
					{#if meshWarnings[node.id]}
						{@const mw = meshWarnings[node.id]}
						<button
							type="button"
							class="inline-flex shrink-0 items-center rounded px-0.5 {selected
								? ''
								: 'text-warning'} hover:bg-overlay"
							data-testid="object-mesh-warning"
							data-open-edges={mw.openEdges}
							data-tip={mw.tooltip}
							aria-label={$t('Mesh errors of {name}: {details}', {
								name: node.name,
								details: mw.tooltip.replace(/\s+/g, ' ')
							})}
							onclick={() => onrepair?.(node.id)}
						>
							<IconWrench class="h-3.5 w-3.5" aria-hidden="true" />
						</button>
					{/if}
					{#if cutLinks[node.id]}
						<span
							class="inline-flex shrink-0 items-center opacity-80"
							data-testid="object-cut-info"
							data-tip={$t(
								'Cut part, linked to {n} part(s): grouped selection, uniform scale only. Object menu › “Invalidate cut info” to unlink it.',
								{ n: cutLinks[node.id] }
							)}
							role="img"
							aria-label={$t('Cut part linked to {n} part(s)', { n: cutLinks[node.id] })}
						>
							<IconScissors class="h-3.5 w-3.5" aria-hidden="true" />
						</span>
					{/if}
				{/if}

				<!-- Commandes de la ligne : groupées à droite ; si le nom ne tient pas à côté,
				     elles passent sous le nom (flex-wrap) au lieu de le tronquer (« cube2… »). -->
				<div class="ml-auto flex shrink-0 items-center gap-1" data-testid="object-row-controls">
					{#if nSettings || ov?.ranges || ov?.parts}
						<button
							type="button"
							class="flex shrink-0 items-center gap-1.5 rounded px-1 text-2xs tabular-nums opacity-90 hover:bg-overlay"
							data-testid="object-overrides"
							data-settings={nSettings}
							aria-label={$t('Overrides of {name}: {n} setting(s)', {
								name: node.name,
								n: nSettings
							}) +
								(ov?.ranges ? ', ' + $t('{n} height range(s)', { n: ov.ranges }) : '') +
								(ov?.parts ? ', ' + $t('{n} part(s)/modifier(s)', { n: ov.parts }) : '')}
							data-tip={$t('{n} overridden setting(s)', { n: nSettings }) +
								(ov?.ranges ? ' · ' + $t('{n} height range(s)', { n: ov.ranges }) : '') +
								(ov?.parts ? ' · ' + $t('{n} part(s)/modifier(s)', { n: ov.parts }) : '')}
							onclick={() => onselect(node.id, false)}
						>
							{#if nSettings}<span class="flex items-center gap-0.5"
									><IconSettings class="h-3 w-3" aria-hidden="true" />{nSettings}</span
								>{/if}
							{#if ov?.ranges}<span class="flex items-center gap-0.5"
									><IconLayers class="h-3 w-3" aria-hidden="true" />{ov.ranges}</span
								>{/if}
							{#if ov?.parts}<span class="flex items-center gap-0.5"
									><IconBox class="h-3 w-3" aria-hidden="true" />{ov.parts}</span
								>{/if}
						</button>
					{/if}

					{#if node.kind !== 'group'}
						{@const ex = shownExtruder(node)}
						{#if (ex > 0 || extruders.length === 1) && extruderColors[Math.max(ex, 1) - 1]}
							<span
								class="inline-block h-2.5 w-2.5 shrink-0 rounded-full border border-border-strong"
								style="background-color: {extruderColors[Math.max(ex, 1) - 1]}"
								data-testid="object-extruder-swatch"
								aria-hidden="true"
							></span>
						{/if}
						<!-- Orca masque la colonne filament avec un seul filament
					     (ObjectList::set_filament_column_hidden). -->
						{#if extruders.length > 1}<select
								class="ws-field ws-field-sm h-6 px-0.5"
								aria-label={$t('Extruder of {name}', { name: node.name })}
								value={ex}
								onchange={(e) => onextruder(node.id, Number(e.currentTarget.value))}
							>
								{#if !rootObject(node)}<option value={0}>auto</option>{/if}
								{#each extruders as ex (ex)}
									<option value={ex}>{ex}</option>
								{/each}
							</select>{/if}
					{/if}

					{#if node.kind === 'object' && onassign && plates.length > 1}
						<select
							class="ws-field ws-field-sm h-6 max-w-24 px-0.5"
							aria-label={$t('Plate of {name}', { name: node.name })}
							value={plates.find((plate) => plate.objectIds.includes(node.id))?.id ?? ''}
							onchange={(e) => onassign(node.id, e.currentTarget.value)}
						>
							{#each plates as plate (plate.id)}
								<option value={plate.id}>{plate.name}</option>
							{/each}
						</select>
					{/if}

					<button
						type="button"
						class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-current opacity-80 transition-colors hover:bg-overlay hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
						aria-pressed={node.locked}
						aria-label={$t('Lock {name}', { name: node.name })}
						title={node.locked ? $t('Unlock') : $t('Lock')}
						onclick={() => ontogglelock(node.id)}
					>
						{#if node.locked}<IconLock class="h-3.5 w-3.5" aria-hidden="true" />{:else}<IconUnlock
								class="h-3.5 w-3.5"
								aria-hidden="true"
							/>{/if}
					</button>
					<button
						type="button"
						class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-current opacity-80 transition-colors hover:bg-overlay hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
						aria-pressed={node.hidden}
						aria-label={$t('Hide {name}', { name: node.name })}
						title={node.hidden ? $t('Show') : $t('Hide')}
						onclick={() => ontogglehide(node.id)}
					>
						{#if node.hidden}<IconEyeOff class="h-3.5 w-3.5" aria-hidden="true" />{:else}<IconEye
								class="h-3.5 w-3.5"
								aria-hidden="true"
							/>{/if}
					</button>
					<button
						type="button"
						class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-current opacity-80 transition-colors hover:bg-overlay hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
						aria-label={$t('Duplicate {name}', { name: node.name })}
						title={$t('Duplicate')}
						onclick={() => onduplicate(node.id)}
					>
						<IconCopy class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
					<button
						type="button"
						class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-current opacity-80 transition-colors hover:bg-overlay hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
						aria-label={$t('Delete {name}', { name: node.name })}
						title={$t('Delete')}
						onclick={() => ondelete(node.id)}
					>
						<IconX class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				</div>
				{#if issues[node.id]?.inline}
					<!-- Cause lisible sans survol (ex. échec de conversion du modèle). -->
					<p
						class="w-full pb-0.5 text-2xs leading-snug {selected
							? ''
							: issues[node.id].level === 'error'
								? 'text-danger'
								: 'text-warning-content'}"
						data-testid="object-issue-text"
					>
						{issues[node.id].text}
					</p>
				{/if}
			</div>

			{#if tree.children(node.id).length}
				<ul>
					{#each tree.children(node.id) as child (child.id)}
						{@render row(child, depth + 1)}
					{/each}
				</ul>
			{/if}
		</li>
	{/if}
{/snippet}

<div class="flex flex-col gap-2">
	<div class="flex items-center justify-between">
		<span class="ws-section-title">{$t('Objects')}</span>
		{#if sequential}
			<span
				class="ws-badge mr-auto ml-2"
				data-testid="object-list-sequential"
				data-tip={$t(
					'Print by object: the list order is the print order. Drag an object onto another (or Alt+↑/↓) to change it.'
				)}>{$t('By object||list')}</span
			>
			<button
				type="button"
				class="ws-icon-btn ws-icon-btn-xs mr-1 {showOrderOnScene ? 'text-primary' : ''}"
				aria-pressed={showOrderOnScene}
				aria-label={$t('Print order on the scene')}
				data-tip={showOrderOnScene
					? $t('Hide the order numbers on the scene')
					: $t('Show the order numbers on the scene')}
				data-testid="object-list-order-toggle"
				onclick={() => (showOrderOnScene = !showOrderOnScene)}
				><IconHash class="h-3.5 w-3.5" aria-hidden="true" /></button
			>
		{/if}
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary"
			disabled={selection.size < 2}
			onclick={() => ongroup()}>{$t('Group')}</button
		>
	</div>
	{#if tree.roots().length > 0}
		<!-- Orca « Search plate, object and part. » (Plater m_search_bar). -->
		<label class="relative block">
			<span class="sr-only">{$t('Search plate, object and part')}</span>
			<IconSearch
				class="pointer-events-none absolute top-1/2 left-2 h-3.5 w-3.5 -translate-y-1/2 text-content-subtle"
				aria-hidden="true"
			/>
			<input
				type="search"
				bind:value={query}
				placeholder={$t('Plate, object or part…')}
				class="ws-field ws-field-sm w-full pl-7"
				data-testid="object-search"
				onkeydown={(e) => {
					if (e.key === 'Escape' && query) {
						e.stopPropagation();
						query = '';
					}
				}}
			/>
		</label>
	{/if}
	<ul>
		<!-- Phase 46 (FIX1-a) : instances sous l'objet (sous-liste « Instances »
		     d'Orca) ; en objet par objet, la liste reste l'ordre d'impression. -->
		{#each sequential ? tree
					.roots()
					.map( (node) => ({ node, instances: [] }) ) : instanceListLayout(tree.roots()) as entry (entry.node.id)}
			{#if isVisible(entry.node)}
				{@render row(entry.node, 0)}
			{/if}
			{@const shownInstances = entry.instances.filter(isVisible)}
			{#if shownInstances.length}
				<li
					class="flex items-center gap-1 px-1.5 pl-5 text-xs text-content-subtle"
					data-testid="object-instances"
					data-count={entry.instances.length + 1}
				>
					{$t('Instances')} ({entry.instances.length + 1})
				</li>
				{#each shownInstances as inst (inst.id)}
					{@render row(inst, 1)}
				{/each}
			{/if}
		{/each}
	</ul>
	{#if search}
		{#if shownHere === 0 && elsewhere.length === 0}
			<p class="px-1 text-xs text-content-subtle" data-testid="object-search-empty">
				{$t('No plate, object or part matches “{query}”.', { query: query.trim() })}
			</p>
		{/if}
		{#if elsewhere.length > 0}
			<div class="flex flex-col gap-0.5" data-testid="object-search-elsewhere">
				<span class="px-1 text-2xs text-content-subtle">{$t('On other plates')}</span>
				{#each elsewhere as hit (hit.id)}
					<button
						type="button"
						class="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-left text-sm hover:bg-overlay"
						data-testid="object-search-hit"
						disabled={!onlocate}
						onclick={() => hit.plateId && onlocate?.(hit.plateId, hit.id)}
					>
						<span class="truncate">{hit.name}</span>
						{#if hit.part}<span class="truncate text-xs text-content-subtle">› {hit.part}</span
							>{/if}
						<span class="ws-badge ml-auto shrink-0 normal-case">{hit.plateName}</span>
					</button>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	/* Cible de dépôt pendant le glisser-déposer d'ordre d'impression. */
	.ws-drop-target {
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
	/* Résultat de recherche (hors sélection). */
	.ws-search-hit {
		box-shadow: inset 2px 0 0 0 var(--color-primary);
	}
</style>
