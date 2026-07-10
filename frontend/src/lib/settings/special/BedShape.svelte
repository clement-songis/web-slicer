<script lang="ts">
	// Dialog « forme de plateau » (T045) : édite `printable_area` (rectangle
	// origine, cas courant), la zone d'exclusion, et les fichiers modèle/texture.
	import {
		BED_SHAPE_KEYS,
		bedExtents,
		parsePoints,
		rectangularBed,
		serializePoints
	} from './dialogs';

	interface Props {
		/** Valeurs par clé (liable). */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();

	const extents = $derived(bedExtents(parsePoints(values[BED_SHAPE_KEYS.printableArea])));

	function setSize(width: number, depth: number) {
		values[BED_SHAPE_KEYS.printableArea] = serializePoints(rectangularBed(width, depth));
	}

	const FIELD =
		'w-full rounded border border-gray-300 px-2 py-1 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100';
</script>

<div class="flex flex-col gap-3">
	<div class="grid grid-cols-2 gap-3">
		<label class="text-sm">
			<span class="mb-1 block text-gray-700 dark:text-gray-300">Largeur (mm)</span>
			<input
				type="number"
				min="0"
				value={extents.width}
				oninput={(e) => setSize(Number(e.currentTarget.value), extents.depth)}
				class={FIELD}
			/>
		</label>
		<label class="text-sm">
			<span class="mb-1 block text-gray-700 dark:text-gray-300">Profondeur (mm)</span>
			<input
				type="number"
				min="0"
				value={extents.depth}
				oninput={(e) => setSize(extents.width, Number(e.currentTarget.value))}
				class={FIELD}
			/>
		</label>
	</div>

	<label class="text-sm">
		<span class="mb-1 block text-gray-700 dark:text-gray-300"
			>Zone d'exclusion (points « XxY », séparés par ;)</span
		>
		<input
			type="text"
			value={(
				parsePoints(values[BED_SHAPE_KEYS.excludeArea]).map((p) => `${p.x}x${p.y}`) ?? []
			).join(';')}
			oninput={(e) =>
				(values[BED_SHAPE_KEYS.excludeArea] = serializePoints(parsePoints(e.currentTarget.value)))}
			placeholder="ex. 0x0;20x20"
			class={FIELD}
		/>
	</label>

	<label class="text-sm">
		<span class="mb-1 block text-gray-700 dark:text-gray-300">Modèle de plateau (fichier)</span>
		<input
			type="text"
			value={String(values[BED_SHAPE_KEYS.customModel] ?? '')}
			oninput={(e) => (values[BED_SHAPE_KEYS.customModel] = e.currentTarget.value)}
			class={FIELD}
		/>
	</label>

	<label class="text-sm">
		<span class="mb-1 block text-gray-700 dark:text-gray-300">Texture de plateau (fichier)</span>
		<input
			type="text"
			value={String(values[BED_SHAPE_KEYS.customTexture] ?? '')}
			oninput={(e) => (values[BED_SHAPE_KEYS.customTexture] = e.currentTarget.value)}
			class={FIELD}
		/>
	</label>
</div>
