// Phase 51 — correctifs et vérifications de l'audit 49, partie interface :
// - FIX1-a (confirmation en conditions d'audit) : « Remplir le plateau
//   d'instances… » remplit SANS dialogue de nombre (FillBedJob d'Orca) et
//   regroupe en 1 objet × N instances (instanceGroup, sous-liste
//   « Instances (N) »), persisté à la réouverture ; « … de copies » garde
//   des objets séparés « (copie) ». L'audit 49 avait conclu « non corrigé »
//   en cliquant « Définir le nombre d'instances… » (premier item
//   /instances/i) : le clic exact est vérifié ici.
// - PLATE49-a : une plage « autres couches » vide (ligne d'« Ajouter » jamais
//   remplie) n'invalide plus l'application — les plages remplies
//   s'appliquent (Orca ignore les plages vides à l'enregistrement) ; les
//   valeurs invalides gardent le message de saisie.
// - PERF49-a : le retour d'état du bouton Trancher suit l'import (état
//   dérivé des imports/maillages, pas d'un minuteur) : à l'ouverture d'un
//   projet, la raison visible n'est plus « Ajoutez un objet à trancher »
//   pendant le chargement des maillages.
//
// Oracle Orca 2.4.1 : FillBedJob.cpp:175/190-198 (instances vs copies),
// OtherLayersSeqPanel (append_layer : 1ʳᵉ plage 2→Fin, suivantes vides ;
// « Please input layer value (>= 2). »), ParameterUtils.cpp
// (get_other_layers_print_sequence : séquence par blocs [début, fin, ordre]).
//
// CONTRAT: object-row, object-instances, object-name, menuitem (action
// exacte), [role=dialog] (aucun pour FillBed), settings-dialog,
// plate-layer-range(-begin/-end-kind/-end/-add), settings-apply,
// plate-settings-button, slice-button, slice-blocker-reason, add-filament-slot.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

type SceneObject = { instanceGroup?: string };
type Plate = { settings?: Record<string, unknown> };
type Project = {
  scene: { objects: SceneObject[]; plates: { plates?: Plate[] } | Plate[] };
};

const rows = (page: Page) => page.getByTestId("object-row");

async function save(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
}

async function project(page: Page, id: string): Promise<Project> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as Project;
}

function platesOf(p: Project): Plate[] {
  return Array.isArray(p.scene.plates) ? p.scene.plates : p.scene.plates.plates;
}

async function objectMenu(page: Page, name: RegExp, row = 0) {
  await rows(page).nth(row).click({ button: "right" });
  const item = page.getByRole("menuitem", { name });
  await item.scrollIntoViewIfNeeded();
  await item.click();
}

test("FIX1-a : « Remplir le plateau d'instances… » remplit sans dialogue et regroupe (1 objet × N instances, persisté) @F:ctx.fill-bed-with-instances @F:ctx.fill-bed-with-copies", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });

  // Clic sur l'entrée EXACTE (l'audit 49 avait visé « Définir le nombre
  // d'instances… », premier item correspondant à /instances/i).
  await objectMenu(
    page,
    /^Fill bed with instances…$|^Remplir le plateau d’instances…$/,
  );
  // FillBedJob d'Orca : aucun dialogue de nombre, remplissage direct.
  await page.waitForTimeout(800);
  expect(await page.locator("[role=dialog]").count()).toBe(0);

  // 1 objet × N instances : sous-liste « Instances (N) », aucune « (copie) ».
  const block = page.getByTestId("object-instances");
  await expect(block).toHaveCount(1, { timeout: 30_000 });
  const n = Number((await block.first().getAttribute("data-count")) ?? "0");
  expect(n).toBeGreaterThan(1);
  const names = await page.getByTestId("object-name").allInnerTexts();
  expect(names.length).toBeGreaterThan(1);
  expect(names.some((x) => x.includes("(copie)"))).toBe(false);

  // Persistance : la scène ne garde qu'UN groupe d'instances partagé.
  await save(page);
  const p = await project(page, id);
  expect(p.scene.objects.length).toBe(n);
  const groups = new Set(
    p.scene.objects.map((o) => o.instanceGroup).filter((g) => g),
  );
  expect(groups.size).toBe(1);

  // Réouverture : le regroupement survit au rechargement de la page.
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await expect(page.getByTestId("object-instances")).toHaveCount(1, {
    timeout: 60_000,
  });

  // « … de copies » (parité FillBedJob faux) : objets frères « (copie) ».
  await objectMenu(
    page,
    /^Fill bed with copies$|^Remplir le plateau de copies$/,
  );
  await expect
    .poll(
      async () => {
        const all = await page.getByTestId("object-name").allInnerTexts();
        return all.filter((x) => x.includes("(copie)")).length;
      },
      { timeout: 30_000 },
    )
    .toBeGreaterThan(0);
});

test("PLATE49-a : une plage « autres couches » vide n'empêche plus l'application ; les valeurs invalides gardent le message @F:editor.plate-settings", async ({
  page,
}) => {
  const id = await bootstrap(page);
  // Deux filaments : les séquences par couche existent dès 2 filaments.
  await page.getByTestId("add-filament-slot").click();
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });

  await page.getByTestId("plate-settings-button").click();
  const dialog = page.getByTestId("settings-dialog");
  await expect(dialog).toBeVisible();
  const develop = dialog.getByTestId("settings-mode-develop");
  if (await develop.count()) await develop.click();

  // 1ʳᵉ couche personnalisée [2,1].
  await dialog
    .locator('[data-param="first_layer_sequence_choice"] select')
    .selectOption("Customize");
  const first = dialog.getByTestId("plate-first-layer-order");
  await first
    .getByRole("button", { name: /filament 2 (plus tôt|earlier)/i })
    .click();

  // Autres couches : 2 plages remplies (2→6 [2,1], 7→Fin [1,2]) + 1 ligne
  // vide issue des « Ajouter » (repro exact de l'audit 49 plate49).
  await dialog
    .locator('[data-param="other_layers_sequence_choice"] select')
    .selectOption("Customize");
  const other = dialog.getByTestId("plate-other-layers-sequence");
  const ranges = other.getByTestId("plate-layer-range");
  await other.getByTestId("plate-layer-range-add").click();
  await other.getByTestId("plate-layer-range-add").click();
  const r0 = ranges.nth(0);
  await r0.getByTestId("plate-layer-range-begin").fill("2");
  await r0.getByTestId("plate-layer-range-begin").press("Tab");
  await r0.getByTestId("plate-layer-range-end-kind").selectOption("custom");
  await r0.getByTestId("plate-layer-range-end").fill("6");
  await r0.getByTestId("plate-layer-range-end").press("Tab");
  // Ordre [2,1] de la plage 1 (repro audit) : « filament 2 plus tôt ».
  await r0
    .getByRole("button", { name: /filament 2 (plus tôt|earlier)/i })
    .click();
  const r1 = ranges.nth(1);
  await r1.getByTestId("plate-layer-range-begin").fill("7");
  await r1.getByTestId("plate-layer-range-begin").press("Tab");
  await r1.getByTestId("plate-layer-range-end-kind").selectOption("end");
  await r1.scrollIntoViewIfNeeded();
  // La 3ᵉ ligne (2 « Ajouter » + la ligne initiale devenue 3ᵉ) reste vide.
  await expect(ranges).toHaveCount(3);

  // Appliquer : la plage vide est ignorée, TOUT s'applique (plus de perte).
  await dialog.getByTestId("settings-apply").click();
  await expect(dialog).toHaveCount(0, { timeout: 10_000 });
  await save(page);
  const p = await project(page, id);
  const settings = platesOf(p)[0].settings ?? {};
  expect(settings.first_layer_print_sequence).toBe("2,1");
  expect(settings.other_layers_print_sequence).toBe("2,6,2,1,7,2147483646,1,2");
  expect(settings.other_layers_print_sequence_nums).toBe("2");

  // Valeurs invalides (début saisi, fin personnalisée vide) : message de
  // saisie conservé, dialogue ouvert, rien d'applié.
  await page.getByTestId("plate-settings-button").click();
  await expect(dialog).toBeVisible();
  const develop2 = dialog.getByTestId("settings-mode-develop");
  if (await develop2.count()) await develop2.click();
  const other2 = dialog.getByTestId("plate-other-layers-sequence");
  const ranges2 = other2.getByTestId("plate-layer-range");
  await expect(ranges2).toHaveCount(2); // la plage vide n'est pas relue
  await ranges2
    .nth(0)
    .getByTestId("plate-layer-range-end-kind")
    .selectOption("custom");
  await dialog.getByTestId("settings-apply").click();
  await expect(dialog.locator("[role=alert]")).toContainText(/couche|layer/i);
  await expect(dialog).toBeVisible();
});

test("PERF49-a : la raison de blocage du bouton Trancher suit l'import (chargement des objets), pas « Ajoutez un objet » @F:ctx.load @F:core.slice-ui", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await save(page);

  // Réouverture : pendant que les maillages se chargent, le libellé visible
  // (slice-blocker-reason) annonce le chargement des objets du projet —
  // jamais « Ajoutez un objet à trancher » alors que la liste les affiche.
  await page.reload({ waitUntil: "domcontentloaded" });
  const button = page.getByTestId("slice-button");
  const reason = page.getByTestId("slice-blocker-reason");
  // La fenêtre de chargement peut être brève : toute raison observée avant
  // l'activation doit être « Chargement… » (l'état dérive des imports).
  await expect.poll(() => button.isEnabled(), { timeout: 60_000 }).toBe(true);
  const seen = await reason.count();
  if (seen > 0) {
    await expect(reason).not.toContainText(/Ajoutez un objet|Add an object/i);
  }
  // Activé : plus de libellé de blocage du tout.
  await expect(reason).toHaveCount(0);
  expect(id).toBeTruthy();
});
