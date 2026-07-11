# Icônes custom

Icônes propres à Web-Slicer, absentes des sets Iconify. Chaque fichier `*.svg`
de ce dossier est exposé par [unplugin-icons](https://github.com/unplugin/unplugin-icons)
comme composant Svelte via la collection locale `custom` (configurée dans
`vite.config.ts`).

## Utilisation

```svelte
<script lang="ts">
	import IconBuildPlate from '~icons/custom/build-plate';
	import IconNozzle from '~icons/custom/nozzle';
	// …et n'importe quelle icône Iconify installée, ex. Lucide :
	import IconSun from '~icons/lucide/sun';
</script>

<IconBuildPlate class="h-5 w-5 text-content-muted" />
<IconNozzle class="h-5 w-5" />
<IconSun />
```

Le nom du fichier (`build-plate.svg`) donne le nom d'import (`~icons/custom/build-plate`).

## Convention

- **`viewBox="0 0 24 24"`** (grille Lucide) pour l'homogénéité visuelle.
- **`currentColor`** obligatoire : utiliser `stroke="currentColor"` (traits) ou
  `fill="currentColor"` (aplats), jamais de couleur en dur — l'icône hérite
  ainsi de la couleur de texte et des variables de thème.
- Pas de dimensions fixes (`width`/`height`) : la taille est pilotée par les
  classes utilitaires (`h-5 w-5`) au point d'usage.

## Ajouter une icône

Déposer le SVG ici : il est disponible immédiatement (HMR en dev). Le typage
`~icons/custom/*` est fourni par `unplugin-icons/types/svelte` (voir `tsconfig.json`).
