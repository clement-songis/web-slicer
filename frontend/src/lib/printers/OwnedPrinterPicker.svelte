<script lang="ts">
	// Sélecteur d'imprimante de l'éditeur (Phase 14, refonte) : contrairement à la
	// cascade marque → modèle → buse du catalogue, on ne montre ici que les
	// imprimantes **possédées** par l'utilisateur — une liste simple par nom, sans
	// marque — puis la **buse** parmi les variantes du modèle de l'imprimante
	// choisie. Deux actions à côté : « ＋ » (ajouter une imprimante, ouvre la
	// grille) et « ✎ » (éditer les réglages, ouvre le dialogue). Aucun accès réseau.
	import type { PrinterCatalogModel, PrinterCatalogVendor, PrinterResponse } from '$lib/api/types';
	import IconAdd from '~icons/lucide/plus';
	import IconEdit from '~icons/lucide/pencil';

	interface Props {
		/** Imprimantes déclarées du compte. */
		printers: PrinterResponse[];
		/** Catalogue complet (marque → modèle → buses) pour résoudre le modèle. */
		catalog: PrinterCatalogVendor[];
		/** Preset machine actif (= buse retenue) ; liable, piloté par les deux menus. */
		selectedMachinePresetId?: string | null;
		/** Ouvre la grille d'ajout d'imprimante. */
		onadd: () => void;
		/** Ouvre le dialogue de réglages de l'imprimante. */
		onedit: () => void;
	}

	let {
		printers,
		catalog,
		selectedMachinePresetId = $bindable(null),
		onadd,
		onedit
	}: Props = $props();

	// Tous les modèles du catalogue, aplatis (marque incluse dans chaque modèle).
	const models = $derived(catalog.flatMap((v) => v.models));

	/** Modèle contenant une variante de ce preset machine (ou null). */
	function modelOf(presetId: string | null | undefined): PrinterCatalogModel | null {
		if (!presetId) return null;
		return models.find((m) => m.variants.some((v) => v.machine_preset_id === presetId)) ?? null;
	}
	function modelKeyOf(presetId: string | null | undefined): string | null {
		const m = modelOf(presetId);
		return m ? `${m.vendor}::${m.model}` : null;
	}

	// Imprimante possédée actuellement pointée par le menu (état local : le preset
	// actif seul ne suffit pas à lever l'ambiguïté quand deux imprimantes partagent
	// un modèle). Réaligné sur le preset actif tant qu'il n'est pas déjà cohérent.
	let selectedPrinterId = $state<string | null>(null);
	$effect(() => {
		if (printers.some((p) => p.id === selectedPrinterId)) return;
		// Priorité : l'imprimante dont la buse déclarée est exactement la buse
		// active ; sinon celle du même modèle ; sinon la première déclarée.
		const key = modelKeyOf(selectedMachinePresetId);
		const inferred =
			printers.find((p) => p.machine_preset_id === selectedMachinePresetId) ??
			printers.find((p) => modelKeyOf(p.machine_preset_id) === key) ??
			printers[0];
		selectedPrinterId = inferred?.id ?? null;
	});

	const selectedPrinter = $derived(printers.find((p) => p.id === selectedPrinterId) ?? null);
	const selectedModel = $derived(
		selectedPrinter ? modelOf(selectedPrinter.machine_preset_id) : null
	);
	const nozzles = $derived(selectedModel?.variants ?? []);

	// Change d'imprimante : réinitialise la buse sur celle déclarée pour elle.
	function onPrinterChange(id: string) {
		selectedPrinterId = id;
		const printer = printers.find((p) => p.id === id);
		if (printer) selectedMachinePresetId = printer.machine_preset_id;
	}

	const FIELD =
		'w-full rounded border border-border-strong px-2 py-1 text-sm bg-surface-raised text-content';
	const ICON_BTN =
		'flex shrink-0 items-center justify-center rounded border border-border-strong px-2 py-1 text-content-muted hover:bg-overlay disabled:opacity-40';
</script>

<div class="flex flex-col gap-2">
	<div class="flex items-center gap-1">
		<select
			class={FIELD}
			aria-label="Imprimante"
			value={selectedPrinterId}
			onchange={(e) => onPrinterChange(e.currentTarget.value)}
		>
			{#each printers as p (p.id)}
				<option value={p.id}>{p.name}</option>
			{/each}
		</select>
		<button type="button" class={ICON_BTN} title="Ajouter une imprimante…" onclick={onadd}>
			<IconAdd />
		</button>
		<button
			type="button"
			class={ICON_BTN}
			title="Réglages de l'imprimante"
			onclick={onedit}
			disabled={!selectedPrinter}
		>
			<IconEdit />
		</button>
	</div>

	{#if nozzles.length > 0}
		<select class={FIELD} aria-label="Buse" bind:value={selectedMachinePresetId}>
			{#each nozzles as v (v.machine_preset_id)}
				<option value={v.machine_preset_id}>{v.nozzle} mm</option>
			{/each}
		</select>
	{/if}
</div>
