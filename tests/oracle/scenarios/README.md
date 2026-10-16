# Scénarios (gizmos, barre d'outils, menus, raccourcis)

Un fichier par feature : `<id>.spec.ts`, écrit par une session « auteur de
tests », gelé ensuite. Règles :

1. Le titre de chaque test contient le tag `@F:<id>`.
2. On vérifie un **résultat observable**, jamais seulement « le bouton existe » :
   - de préférence le G-code exporté (`downloadGcode`) ou le projet 3MF
     exporté (`GET /api/projects/{id}/export/3mf`), comparé à ce que produit
     Orca pour la même opération (lire le code d'Orca pour en déduire
     l'effet attendu : nombre d'objets après une coupe, bloc de peinture dans
     le 3MF, rotation dans la matrice de transformation, etc.) ;
   - sinon un état d'UI précis et vérifiable (attributs `data-*` déclarés).
3. Les nouveaux sélecteurs sont déclarés en tête : `// CONTRAT: data-testid="cut-confirm" …`.
4. Utiliser `../lib/app` et `../lib/gcode` ; pas de `waitForTimeout`.
5. Le test doit **échouer** sur le code actuel si la feature n'existe pas.

Modèle :

```ts
// CONTRAT: data-testid="gizmo-cut", data-testid="cut-height" (input), data-testid="cut-confirm"
import { expect, test } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

test('coupe à mi-hauteur : deux objets de 10 mm @F:gizmo.cut', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('object-row').first().click();
	await page.getByTestId('gizmo-cut').click();
	await page.getByTestId('cut-height').fill('10');
	await page.getByTestId('cut-confirm').click();
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	// + vérifier les hauteurs via le 3MF exporté ou le G-code
});
```
