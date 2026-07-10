<script lang="ts">
	// Liste d'objets de la scène (T053) : arbre objets/pièces/groupes avec
	// sélection, verrouillage/masquage, extrudeur par nœud, duplication et
	// suppression. Purement présentationnel : les mutations passent par des
	// callbacks (le parent applique sur l'ObjectTree et historise).
	import type { ObjectTree, SceneNode } from './objects';

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
		/** Nombre d'extrudeurs de l'imprimante active (pour le sélecteur). */
		extruderCount?: number;
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
		extruderCount = 1
	}: Props = $props();

	const extruders = $derived(Array.from({ length: extruderCount }, (_, i) => i + 1));
</script>

{#snippet row(node: SceneNode, depth: number)}
	{@const selected = selection.has(node.id)}
	<li>
		<div
			class="flex items-center gap-1 rounded px-1 py-0.5 {selected
				? 'bg-slate-600 text-white'
				: 'hover:bg-slate-800'}"
			style="padding-left: {depth * 1 + 0.25}rem"
		>
			<button
				type="button"
				class="flex-1 truncate text-left text-sm {tree.isHidden(node.id) ? 'opacity-40' : ''}"
				onclick={(e) => onselect(node.id, e.shiftKey || e.ctrlKey || e.metaKey)}
			>
				<span class="text-slate-500">{node.kind === 'group' ? '▾' : '•'}</span>
				{node.name}
			</button>

			{#if node.kind !== 'group'}
				<select
					class="rounded border border-slate-600 bg-slate-900 text-xs"
					aria-label={`Extrudeur de ${node.name}`}
					value={node.extruder}
					onchange={(e) => onextruder(node.id, Number(e.currentTarget.value))}
				>
					<option value={0}>auto</option>
					{#each extruders as ex (ex)}
						<option value={ex}>{ex}</option>
					{/each}
				</select>
			{/if}

			<button
				type="button"
				class="px-1 text-xs"
				aria-pressed={node.locked}
				aria-label={`Verrouiller ${node.name}`}
				onclick={() => ontogglelock(node.id)}>{node.locked ? '🔒' : '🔓'}</button
			>
			<button
				type="button"
				class="px-1 text-xs"
				aria-pressed={node.hidden}
				aria-label={`Masquer ${node.name}`}
				onclick={() => ontogglehide(node.id)}>{node.hidden ? '🙈' : '👁'}</button
			>
			<button
				type="button"
				class="px-1 text-xs"
				aria-label={`Dupliquer ${node.name}`}
				onclick={() => onduplicate(node.id)}>⧉</button
			>
			<button
				type="button"
				class="px-1 text-xs"
				aria-label={`Supprimer ${node.name}`}
				onclick={() => ondelete(node.id)}>✕</button
			>
		</div>

		{#if tree.children(node.id).length}
			<ul>
				{#each tree.children(node.id) as child (child.id)}
					{@render row(child, depth + 1)}
				{/each}
			</ul>
		{/if}
	</li>
{/snippet}

<div class="flex flex-col gap-2">
	<div class="flex items-center justify-between">
		<span class="text-sm font-medium text-slate-200">Objets</span>
		<button
			type="button"
			class="rounded border border-slate-600 bg-slate-800 px-2 py-0.5 text-xs hover:bg-slate-700 disabled:opacity-40"
			disabled={selection.size < 2}
			onclick={() => ongroup()}>Grouper</button
		>
	</div>
	<ul>
		{#each tree.roots() as node (node.id)}
			{@render row(node, 0)}
		{/each}
	</ul>
</div>
