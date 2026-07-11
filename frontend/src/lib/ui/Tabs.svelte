<script lang="ts" generics="T extends string">
	// Onglets (T094) — navigation soulignée entre vues d'un même panneau
	// (Objets/Réglages…). `active` est bindable. Présentationnel : le contenu de
	// chaque onglet est rendu par l'appelant selon `active`.
	import { tabClasses } from './styles';

	type Tab = { id: T; label: string };

	type Props = {
		tabs: Tab[];
		active: T;
		ariaLabel?: string;
	};

	let { tabs, active = $bindable(), ariaLabel }: Props = $props();
</script>

<div class="flex border-b border-border" role="tablist" aria-label={ariaLabel}>
	{#each tabs as tab (tab.id)}
		<button
			type="button"
			role="tab"
			aria-selected={active === tab.id}
			onclick={() => (active = tab.id)}
			class={tabClasses(active === tab.id)}
		>
			{tab.label}
		</button>
	{/each}
</div>
