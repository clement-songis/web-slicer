// Phase 34 — les 9 manques d'interface du recompte lot 33 : Ctrl+E (étiquettes),
// Ctrl+7 (vue du plateau courant), Page préc./suiv. (rotation ±45°), « + » / « − »
// (instances ; changement de couleur en Aperçu), recherche de la liste d'objets,
// préférence « Grouper mes presets filament », Graphismes (MSAA, plafond et
// compteur d'images/s), variante de buse du filament (H2D), lien wiki par option.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel, sliceInUi, uniqueEmail } from "../lib/app";

test.use({ locale: "fr-FR" });

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

async function ready(page: Page, models: string[]) {
  await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

test("Ctrl+E, Ctrl+7, Page préc./suiv., + / − en Préparer @F:editor.view-labels-overhang @F:menu.default-view @F:gizmo.rotate @F:ctx.add-instance @F:ctx.remove-instance @F:menu.keyboard-shortcuts", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20"]);
  const scene = page.getByTestId("scene-canvas");

  // Ctrl+E : étiquettes (Orca on_char WXK_CONTROL_E) ; data-labels absent = masquées.
  await expect(scene).not.toHaveAttribute("data-labels");
  await focusEditor(page);
  await page.keyboard.press("Control+e");
  await expect(scene).toHaveAttribute("data-labels", "1");
  await expect(page.getByTestId("object-label")).toContainText("cube20");
  await page.keyboard.press("Control+e");
  await expect(scene).not.toHaveAttribute("data-labels");

  // Ctrl+7 : vue du plateau courant (topfront).
  await page.keyboard.press("Control+7");
  await expect(scene).toHaveAttribute("data-camera-view", "plate");
  await page.keyboard.press("Control+0");
  await expect(scene).toHaveAttribute("data-camera-view", "default");

  // Page préc./suiv. : ±45° autour de Z.
  const rows = page.getByTestId("object-row");
  await rows.first().click();
  await page.getByTestId("tool-rotate").click();
  const rz = page.getByLabel("Rotation (°) Z", { exact: true });
  await focusEditor(page);
  await rows.first().click();
  await page.keyboard.press("PageUp");
  await expect
    .poll(async () => Number(await rz.inputValue()))
    .toBeCloseTo(45, 3);
  await page.keyboard.press("PageDown");
  await page.keyboard.press("PageDown");
  await expect
    .poll(async () => Number(await rz.inputValue()))
    .toBeCloseTo(-45, 3);

  // « + » / « − » : instances ; jamais la dernière.
  await page.keyboard.press("Escape");
  await rows.first().click();
  await page.keyboard.press("+");
  await expect(rows).toHaveCount(2);
  await page.keyboard.press("-");
  await expect(rows).toHaveCount(1);
  await rows.first().click();
  await page.keyboard.press("-");
  await expect(rows).toHaveCount(1);
  await expect(page.getByTestId("editor-message")).toContainText(
    "seule instance",
  );

  // Aide : « Autres raccourcis » liste ces touches.
  await focusEditor(page);
  await page.keyboard.press("?");
  const help = page.getByRole("dialog", { name: "Raccourcis clavier" });
  await expect(
    help.getByRole("region", { name: "Autres raccourcis" }),
  ).toContainText("Ctrl+7");
  await page.keyboard.press("Escape");
});

test("Recherche dans la liste d'objets (plateau, objet, pièce) @F:editor.object-panel", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20", "cylinder"]);
  const rows = page.getByTestId("object-row");
  const search = page.getByTestId("object-search");
  await search.fill("CYL");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toHaveAttribute("data-match", "true");
  await expect(rows.first()).toContainText("cylinder");
  // Ctrl+E dans le champ : pas de raccourci (champ éditable).
  await search.press("Control+e");
  await expect(page.getByTestId("scene-canvas")).not.toHaveAttribute(
    "data-labels",
  );
  await search.fill("introuvable");
  await expect(rows).toHaveCount(0);
  await expect(page.getByTestId("object-search-empty")).toBeVisible();
  await search.press("Escape");
  await expect(search).toHaveValue("");
  await expect(rows).toHaveCount(2);
});

test("Aperçu : « + » / « − » ajoute / retire un changement de couleur, appliqué au tranchage @F:core.preview @F:preview.engine", async ({
  page,
}) => {
  test.setTimeout(400_000);
  await ready(page, ["cube20"]);
  const layers = await sliceInUi(page);
  expect(layers).toBeGreaterThan(20);
  await focusEditor(page);
  await page.keyboard.press("Shift+G");
  const form = page.getByTestId("preview-goto-layer");
  await form.getByRole("spinbutton").fill("20");
  await form.getByRole("spinbutton").press("Enter");
  await expect(page.getByTestId("preview-canvas")).toHaveAttribute(
    "data-layer-high",
    "19",
  );

  await focusEditor(page);
  await page.keyboard.press("+");
  const pending = page.getByTestId("layer-tick-pending");
  await expect(pending).toHaveCount(1);
  await expect(pending.first()).toHaveAttribute("data-type", "color_change");
  await expect(pending.first()).toHaveAttribute("data-layer", "19");
  await expect(page.getByTestId("editor-message")).toContainText("couche 20");
  await expect(page.getByTestId("layer-color-change")).toHaveAttribute(
    "data-active",
    "true",
  );
  await page.keyboard.press("-");
  await expect(pending).toHaveCount(0);
  // Bouton explicite (même action que « + »).
  await page.getByTestId("layer-color-change").click();
  await expect(pending).toHaveCount(1);

  // Retranché : le repère vient du moteur (M600 à la couche 20).
  await sliceInUi(page);
  await expect(
    page.locator('[data-testid="layer-tick"][data-type="color_change"]'),
  ).toHaveCount(1, { timeout: 60_000 });
  await expect(pending).toHaveCount(0);
});

test("Préférences : grouper mes presets filament, Graphismes (MSAA, plafond, compteur) @F:menu.preferences @F:editor.menu-view-prefs", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20"]);
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const dialog = page.getByTestId("preferences-dialog");
  await expect(dialog).toBeVisible();
  await dialog.getByTestId("pref-tab-presets").click();
  await dialog.getByTestId("pref-group-filament-presets").selectOption("type");
  await dialog.getByTestId("pref-tab-graphics").click();
  await dialog.getByTestId("pref-show-fps").check();
  await dialog.getByTestId("pref-fps-cap").selectOption("30");
  await dialog.getByTestId("pref-antialias").uncheck();
  await dialog.getByTestId("preferences-close").click();
  // Compteur visible dans la vue 3D ; la vue recréée sans MSAA reste active.
  await expect(page.getByTestId("fps-counter")).toBeVisible();
  await expect(
    page.getByTestId("scene-canvas").locator("canvas"),
  ).toBeVisible();
  // Persistées pour le compte (serveur) : relues après rechargement.
  await page.waitForTimeout(1200);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await expect(page.getByTestId("fps-counter")).toBeVisible();
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  await dialog.getByTestId("pref-tab-graphics").click();
  await expect(dialog.getByTestId("pref-fps-cap")).toHaveValue("30");
  await expect(dialog.getByTestId("pref-antialias")).not.toBeChecked();
  await dialog.getByTestId("pref-tab-presets").click();
  await expect(dialog.getByTestId("pref-group-filament-presets")).toHaveValue(
    "type",
  );
});

test("Lien wiki au clic sur le libellé d'une option (Orca OptionsGroup::get_url)", async ({
  page,
}) => {
  await ready(page, []);
  await page.getByTestId("open-settings-print").click();
  const dialog = page.getByTestId("settings-dialog");
  await expect(dialog).toBeVisible();
  const link = dialog
    .locator('[data-param="layer_height"]')
    .getByTestId("option-wiki-link");
  await expect(link).toHaveAttribute(
    "href",
    "https://www.orcaslicer.com/wiki/quality_settings_layer_height",
  );
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", /noopener/);
});

test("Filament H2D : sélecteur de variante de buse (Orca m_variant_combo) @F:param.filament_extruder_variant", async ({
  page,
}) => {
  const res = await page.request.post("/api/_harness/bootstrap", {
    data: {
      email: uniqueEmail(),
      password: "harness-password-123",
      printer: "Bambu Lab H2D 0.4 nozzle",
      filament: "Bambu PLA Basic @BBL H2D",
      process: "0.20mm Standard @BBL H2D",
    },
  });
  test.skip(
    res.status() !== 200,
    `presets H2D indisponibles (${res.status()})`,
  );
  const { project_id } = (await res.json()) as { project_id: string };
  await page.goto(`/projects/${project_id}`);
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.getByTestId("open-settings-filament").click();
  const dialog = page.getByTestId("settings-dialog");
  await dialog.getByTestId("settings-mode-develop").click();
  const selector = dialog.getByTestId("filament-variant-selector");
  await expect(selector).toBeVisible({ timeout: 30_000 });
  await expect(selector.getByRole("button")).toHaveText([
    "Entraînement direct : Standard",
    "Entraînement direct : Haut débit",
  ]);
  const row = dialog.locator('[data-param="filament_max_volumetric_speed"]');
  await expect(row).toHaveAttribute("data-variant", "0");
  await expect(row.getByTestId("option-variant-badge")).toBeVisible();
  await dialog.getByTestId("filament-variant-1").click();
  await expect(row).toHaveAttribute("data-variant", "1");
});
