// Parité des paramètres (généré à partir de cases.json, gelé).
// Pour chaque paramètre Orca :
//   « moteur » : même config que la référence Orca → G-code identique.
//   « UI »     : la valeur saisie dans l'UI arrive dans le G-code exactement comme chez Orca.
import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	bootstrap, controlOf, downloadGcode, harnessSlice, importModel, looseEqual, lowerMode, openParam, ORACLE,
	oracleCfg, paramRow, readControl, readJsonRef, readRef, scalar, setControl, sliceInUi, type FlatConfig,
	type ParamCase
} from '../lib/app';
import { configBlock, expectSameGcode } from '../lib/gcode';

const cases = JSON.parse(readFileSync(join(ORACLE, 'params', 'cases.json'), 'utf8')) as ParamCase[];
const byKey = new Map(cases.map((c) => [c.key, c]));
// make_refs.py peut ajuster la valeur pour qu'elle diffère du preset de base : elle fait foi.
for (const c of cases) c.value = readJsonRef<string | string[]>(`param/${c.key}.value.json`) ?? c.value;

for (const c of cases) {
	test.describe(`${c.key} @F:${c.id}`, () => {
		test('moteur', async ({ request }) => {
			test.skip(c.value === null, 'valeur de test manuelle requise (harness/param_overrides.json)');
			const ref = readRef(`param/${c.key}.gcode`);
			const base = readJsonRef<FlatConfig>('base_config.json');
			test.skip(!ref || !base, 'référence Orca absente : lancer tests/oracle/tools/make_refs.py');
			const cfg: FlatConfig = { ...base!, ...(c.requires ?? {}), [c.key]: c.value! };
			expectSameGcode(await harnessSlice(request, oracleCfg.param_model, cfg), ref!, `G-code avec ${c.key}=${JSON.stringify(c.value)}`);
		});

		if (c.ui) {
			test('UI', async ({ page }) => {
				test.skip(c.value === null, 'valeur de test manuelle requise');
				const ref = readRef(`param/${c.key}.gcode`);
				test.skip(!ref, 'référence Orca absente');
				const expected = configBlock(ref!)[c.key];
				expect(expected, `${c.key} absent du bloc de config de la référence Orca`).toBeDefined();
				const scope = c.scope!;
				const fromPreset = scope === 'print' || scope === 'filament' || scope === 'printer';

				await bootstrap(page);
				await importModel(page, oracleCfg.param_model);

				// 1. Visibilité selon le mode Orca : visible à son niveau, masqué au niveau inférieur.
				const below = lowerMode(c.mode);
				if (below) {
					const hidden = await paramRow(page, scope, c.tab, c.key, below);
					await expect(hidden, `${c.key} (niveau ${c.mode}) ne doit pas apparaître en mode ${below}`).toBeHidden();
				}
				const row = await paramRow(page, scope, c.tab, c.key, c.mode);
				await expect(row, `${c.key} doit être visible en mode ${c.mode} (${scope} › ${c.tab})`).toBeVisible();

				// 2. Pré-remplissage par le preset (imprimante / filament / process) : même valeur qu'Orca.
				const base = readJsonRef<FlatConfig>('base_config.json');
				const toggle = row.locator('[data-override-toggle]');
				if (fromPreset && base?.[c.key] !== undefined && !(c.nullable && (await toggle.count()))) {
					const shown = await readControl(await controlOf(row));
					expect(looseEqual(shown, scalar(base[c.key])), `${c.key} pré-rempli : affiché « ${shown} », preset Orca « ${scalar(base[c.key])} »`).toBe(true);
				}

				// 3. Dépendances puis saisie (une option grisée chez Orca l'est aussi ici).
				for (const [k, v] of Object.entries(c.requires ?? {})) {
					const dep = byKey.get(k);
					if (dep?.tab && dep.scope) await setControl(await openParam(page, dep.scope, dep.tab, k), scalar(v));
				}
				const control = await openParam(page, scope, c.tab, c.key);
				const editable = await setControl(control, scalar(c.value!));
				test.skip(!editable, `champ ${c.key} désactivé (dépendance Orca non activée)`);

				// 4. Un preset système modifié est signalé comme tel (astérisque Orca), sans être écrasé.
				if (fromPreset) await expect(page.getByTestId(`preset-select-${scope}`)).toHaveAttribute('data-modified', 'true');
				await page.getByTestId('settings-apply').click();

				// 5. La valeur arrive dans le G-code exactement comme chez Orca.
				await sliceInUi(page);
				const gcode = await downloadGcode(page);
				expect(configBlock(gcode)[c.key], `valeur de ${c.key} dans le G-code exporté depuis l'UI`).toBe(expected);
			});
		}
	});
}
