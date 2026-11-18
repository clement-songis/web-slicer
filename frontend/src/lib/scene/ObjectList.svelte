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
	import type { Plate } from './plates.svelte';

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
		showOrderOnScene = $bindable(true)
	}: Props = $props();

	// Objets racines réordonnables, dans l'ordre (= ordre d'impression).
	const orderable = (node: SceneNode) => node.kind === 'object' && node.parentId === null;
	const printRank = $derived.by(() => {
		const rank: Record<string, number> = {};
		let n = 0;
		for (const node of tree.roots())
			if (orderable(node) && isVisible(node) && tree.isPrintable(node.id)) rank[node.id] = ++n;
		return rank;
	});
	let dragId = $state<string | null>(null);
	let dropId = $state<string | null>(null);

	function neighbour(id: string, delta: -1 | 1): string | null {
		const ids = tree
			.roots()
			.filter((n) => orderable(n) && isVisible(n))
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
		return node.extruder > 0 && node.extruder <= extruders.length ? node.extruder : 0;
	}

	function isVisible(node: SceneNode): boolean {
		if (!visibleIds || visibleIds.has(node.id)) return true;
		return tree.children(node.id).some(isVisible);
	}
</script>

{#snippet row(node: SceneNode, depth: number)}
	{@const selected = selection.has(node.id)}
	{@const ov = overrides[node.id]}
	{@const nSettings = ov?.settings ?? Object.keys(node.settings ?? {}).length}
	{#if isVisible(node)}
		<li>
			<div
				class="flex min-h-8 items-center gap-1 rounded-md px-1.5 py-0.5 transition-colors {selected
					? 'bg-primary text-primary-content'
					: 'hover:bg-overlay'}"
				data-testid="object-row"
				data-printable={node.printable}
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
						aria-label={`Nouveau nom de ${node.name}`}
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
							data-tip="Imprimé en {printRank[node.id]}{printRank[node.id] === 1
								? 'er'
								: 'e'} (objet par objet) · glissez ou Alt+↑/↓ pour changer l'ordre"
							aria-label="Ordre d'impression : {printRank[node.id]}">{printRank[node.id]}</span
						>
					{/if}
					<button
						type="button"
						class="flex-1 truncate text-left text-sm {tree.isHidden(node.id) ||
						!tree.isPrintable(node.id)
							? 'opacity-40'
							: ''}"
						data-testid="object-name"
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
				{/if}

				{#if nSettings || ov?.ranges || ov?.parts}
					<button
						type="button"
						class="flex shrink-0 items-center gap-1.5 rounded px-1 text-2xs tabular-nums opacity-90 hover:bg-overlay"
						data-testid="object-overrides"
						data-settings={nSettings}
						aria-label="Surcharges de {node.name} : {nSettings} réglage(s){ov?.ranges
							? `, ${ov.ranges} plage(s) de hauteur`
							: ''}{ov?.parts ? `, ${ov.parts} pièce(s)/modificateur(s)` : ''}"
						data-tip="{nSettings} réglage(s) surchargé(s){ov?.ranges
							? ` · ${ov.ranges} plage(s) de hauteur`
							: ''}{ov?.parts ? ` · ${ov.parts} pièce(s)/modificateur(s)` : ''}"
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
					{#if ex > 0 && extruderColors[ex - 1]}
						<span
							class="inline-block h-2.5 w-2.5 shrink-0 rounded-full border border-border-strong"
							style="background-color: {extruderColors[ex - 1]}"
							data-testid="object-extruder-swatch"
							aria-hidden="true"
						></span>
					{/if}
					<select
						class="ws-field ws-field-sm h-6 px-0.5"
						aria-label={`Extrudeur de ${node.name}`}
						value={ex}
						onchange={(e) => onextruder(node.id, Number(e.currentTarget.value))}
					>
						<option value={0}>auto</option>
						{#each extruders as ex (ex)}
							<option value={ex}>{ex}</option>
						{/each}
					</select>
				{/if}

				{#if node.kind === 'object' && onassign && plates.length > 1}
					<select
						class="ws-field ws-field-sm h-6 max-w-24 px-0.5"
						aria-label={`Plateau de ${node.name}`}
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
					aria-label={`Verrouiller ${node.name}`}
					title={node.locked ? 'Déverrouiller' : 'Verrouiller'}
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
					aria-label={`Masquer ${node.name}`}
					title={node.hidden ? 'Afficher' : 'Masquer'}
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
					aria-label={`Dupliquer ${node.name}`}
					title="Dupliquer"
					onclick={() => onduplicate(node.id)}
				>
					<IconCopy class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
				<button
					type="button"
					class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-current opacity-80 transition-colors hover:bg-overlay hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
					aria-label={`Supprimer ${node.name}`}
					title="Supprimer"
					onclick={() => ondelete(node.id)}
				>
					<IconX class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
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
		<span class="ws-section-title">Objets</span>
		{#if sequential}
			<span
				class="ws-badge mr-auto ml-2"
				data-testid="object-list-sequential"
				data-tip="Impression objet par objet : l'ordre de la liste est l'ordre d'impression. Glissez un objet sur un autre (ou Alt+↑/↓) pour le changer."
				>Objet par objet</span
			>
			<button
				type="button"
				class="ws-icon-btn ws-icon-btn-xs mr-1 {showOrderOnScene ? 'text-primary' : ''}"
				aria-pressed={showOrderOnScene}
				aria-label="Ordre d'impression sur la scène"
				data-tip={showOrderOnScene
					? "Masquer les numéros d'ordre sur la scène"
					: "Afficher les numéros d'ordre sur la scène"}
				data-testid="object-list-order-toggle"
				onclick={() => (showOrderOnScene = !showOrderOnScene)}
				><IconHash class="h-3.5 w-3.5" aria-hidden="true" /></button
			>
		{/if}
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary"
			disabled={selection.size < 2}
			onclick={() => ongroup()}>Grouper</button
		>
	</div>
	<ul>
		{#each tree.roots() as node (node.id)}
			{#if isVisible(node)}
				{@render row(node, 0)}
			{/if}
		{/each}
	</ul>
</div>

<style>
	/* Cible de dépôt pendant le glisser-déposer d'ordre d'impression. */
	.ws-drop-target {
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
</style>
