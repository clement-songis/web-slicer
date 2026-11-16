<script lang="ts">
	// Info-bulles stylées de l'application (design system : .ws-tooltip, surface
	// inversée, contraste 13,9:1 / 16,3:1). Une seule bulle, montée une fois
	// dans le layout racine ; tout élément portant `data-tip="texte"` (et
	// optionnellement `data-tip-side`) en bénéficie, au survol (après 350 ms)
	// et au focus clavier (immédiat, WCAG 1.4.13 : visible tant que le focus
	// reste, Échap la ferme). Pendant l'affichage, l'élément est décrit par la
	// bulle (`aria-describedby`). Pas d'attribut `title` en parallèle (pas de
	// double bulle native).
	import { onMount, tick } from 'svelte';
	import { tooltipPosition, type TipSide } from './tooltip-position';

	const ID = 'ws-tooltip-layer';
	let text = $state('');
	let pos = $state({ left: -9999, top: -9999 });
	let bubble: HTMLDivElement | undefined = $state();
	let anchor: HTMLElement | null = null;
	let timer: ReturnType<typeof setTimeout> | undefined;

	function hide() {
		clearTimeout(timer);
		if (anchor) {
			const ids = (anchor.getAttribute('aria-describedby') ?? '')
				.split(' ')
				.filter((x) => x && x !== ID);
			if (ids.length) anchor.setAttribute('aria-describedby', ids.join(' '));
			else anchor.removeAttribute('aria-describedby');
		}
		anchor = null;
		text = '';
	}

	async function show(el: HTMLElement) {
		const tip = el.getAttribute('data-tip');
		if (!tip) return;
		anchor = el;
		text = tip;
		const described = el.getAttribute('aria-describedby') ?? '';
		if (!described.split(' ').includes(ID))
			el.setAttribute('aria-describedby', (described + ' ' + ID).trim());
		await tick();
		if (!bubble || anchor !== el) return;
		const a = el.getBoundingClientRect();
		const b = bubble.getBoundingClientRect();
		const p = tooltipPosition(
			{ left: a.left, top: a.top, width: a.width, height: a.height },
			{ width: b.width, height: b.height },
			{ width: window.innerWidth, height: window.innerHeight },
			(el.getAttribute('data-tip-side') as TipSide | null) ?? 'top'
		);
		pos = { left: p.left, top: p.top };
	}

	const target = (e: Event) =>
		e.target instanceof Element ? e.target.closest<HTMLElement>('[data-tip]') : null;

	onMount(() => {
		const over = (e: PointerEvent) => {
			const el = target(e);
			if (!el || el === anchor) return;
			clearTimeout(timer);
			hide();
			timer = setTimeout(() => show(el), 350);
		};
		const out = (e: PointerEvent) => {
			const el = target(e);
			if (!el) return;
			const to = e.relatedTarget instanceof Element ? e.relatedTarget.closest('[data-tip]') : null;
			if (to !== el) hide();
		};
		const focusIn = (e: FocusEvent) => {
			const el = target(e);
			if (el && el.matches(':focus-visible')) void show(el);
		};
		// focusout est aussi émis quand Svelte retire l'élément focalisé du DOM
		// (fermeture d'un dialogue) : écrire un $state pendant ce démontage est
		// interdit (state_unsafe_mutation) → masquage différé d'une microtâche.
		const focusOut = () => queueMicrotask(hide);
		const key = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && anchor) hide();
		};
		document.addEventListener('pointerover', over);
		document.addEventListener('pointerout', out);
		document.addEventListener('focusin', focusIn);
		document.addEventListener('focusout', focusOut);
		document.addEventListener('pointerdown', hide, true);
		document.addEventListener('keydown', key, true);
		window.addEventListener('scroll', hide, true);
		return () => {
			clearTimeout(timer);
			document.removeEventListener('pointerover', over);
			document.removeEventListener('pointerout', out);
			document.removeEventListener('focusin', focusIn);
			document.removeEventListener('focusout', focusOut);
			document.removeEventListener('pointerdown', hide, true);
			document.removeEventListener('keydown', key, true);
			window.removeEventListener('scroll', hide, true);
		};
	});
</script>

{#if text}
	<div
		bind:this={bubble}
		id={ID}
		role="tooltip"
		class="ws-tooltip fixed z-[100] max-w-64 whitespace-nowrap"
		style:left="{pos.left}px"
		style:top="{pos.top}px"
		data-testid="tooltip"
	>
		{text}
	</div>
{/if}
