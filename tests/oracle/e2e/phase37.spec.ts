// Phase 37 — manques d'interface de l'audit (LOT36 §2) et du lot 37 : menu du
// curseur de couches (IMSlider : pause, G-code perso, modifier / supprimer),
// « Changer le type » d'une pièce, tableau des paramètres d'objets, « Fusionner
// avec » un filament, plateau déplacé en premier, navigateur 3D, débit de buse
// (H2D), informations de coupe (invalider), erreurs de maillage (icône →
// réparer), qualité Draco transmise à l'export DRC.
import { expect, test, type Page, type Request } from "@playwright/test";
import { bootstrap, importModel, sliceInUi, uniqueEmail } from "../lib/app";

test.use({ locale: "fr-FR" });

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  return id;
}

async function saveNow(page: Page) {
  const [req] = await Promise.all([
    page.waitForRequest(
      (r: Request) => r.method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  const res = await req.response();
  expect(res?.status()).toBeLessThan(300);
}

/** STL binaire d'un cube de 20 mm auquel il manque une facette (3 arêtes ouvertes). */
function brokenCube(): Buffer {
  const v = (x: number, y: number, z: number) => [x * 20, y * 20, z * 20];
  const quads = [
    [v(0, 0, 0), v(1, 0, 0), v(1, 1, 0), v(0, 1, 0)],
    [v(0, 0, 1), v(0, 1, 1), v(1, 1, 1), v(1, 0, 1)],
    [v(0, 0, 0), v(0, 0, 1), v(1, 0, 1), v(1, 0, 0)],
    [v(0, 1, 0), v(1, 1, 0), v(1, 1, 1), v(0, 1, 1)],
    [v(0, 0, 0), v(0, 1, 0), v(0, 1, 1), v(0, 0, 1)],
    [v(1, 0, 0), v(1, 0, 1), v(1, 1, 1), v(1, 1, 0)],
  ];
  const tris = quads.flatMap(([a, b, c, d]) => [[a, c, b], [a, d, c]]).slice(1);
  const buf = Buffer.alloc(84 + tris.length * 50);
  buf.writeUInt32LE(tris.length, 80);
  tris.forEach((t, i) => {
    const o = 84 + i * 50 + 12;
    t.forEach((p, k) => p.forEach((c, j) => buf.writeFloatLE(c, o + k * 12 + j * 4)));
  });
  return buf;
}

test("Curseur de couches : pause, G-code perso (modifier, supprimer), appliqués au tranchage @F:core.preview @F:preview.engine", async ({
  page,
}) => {
  test.setTimeout(420_000);
  await ready(page, ["cube20"]);
  await sliceInUi(page);
  await focusEditor(page);
  await page.keyboard.press("Shift+G");
  const form = page.getByTestId("preview-goto-layer");
  await form.getByRole("spinbutton").fill("10");
  await form.getByRole("spinbutton").press("Enter");
  await expect(page.getByTestId("preview-canvas")).toHaveAttribute("data-layer-high", "9");

  const menu = page.getByRole("menu");
  const open = async () => {
    await page.getByTestId("layer-code-menu-open").click();
    await expect(menu).toBeVisible();
  };
  // Couche libre : entrées d'ajout d'IMSlider (un seul filament : pas de changement de filament).
  await open();
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Ajouter une pause",
    "Ajouter un G-code personnalisé",
    "Ajouter un changement de couleur",
  ]);
  await menu.getByRole("menuitem", { name: "Ajouter une pause" }).click();
  const pending = page.getByTestId("layer-tick-pending");
  await expect(pending).toHaveCount(1);
  await expect(pending.first()).toHaveAttribute("data-type", "pause_print");
  await expect(pending.first()).toHaveAttribute("data-layer", "9");
  await expect(page.getByTestId("editor-message")).toContainText("Pause ajouté à la couche 10");

  // Couche occupée : seulement la suppression (render_edit_menu).
  await open();
  await expect(menu.getByRole("menuitem")).toHaveText(["Supprimer la pause"]);
  await menu.getByRole("menuitem").click();
  await expect(pending).toHaveCount(0);

  // G-code perso : fenêtre de saisie, puis modification du texte.
  await open();
  await menu.getByRole("menuitem", { name: "Ajouter un G-code personnalisé" }).click();
  const dialog = page.getByTestId("custom-gcode-dialog");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("custom-gcode-ok")).toBeDisabled();
  await page.getByTestId("custom-gcode-input").fill("M117 phase37");
  await page.getByTestId("custom-gcode-ok").click();
  await expect(dialog).toHaveCount(0);
  await expect(pending.first()).toHaveAttribute("data-type", "custom");
  // Clic droit sur la poignée haute : même menu, en édition.
  await page.getByRole("button", { name: "Poignée haute" }).click({ button: "right" });
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Modifier le G-code personnalisé",
    "Supprimer le G-code personnalisé",
  ]);
  await menu.getByRole("menuitem", { name: "Modifier le G-code personnalisé" }).click();
  await expect(page.getByTestId("custom-gcode-input")).toHaveValue("M117 phase37");
  await page.getByTestId("custom-gcode-input").fill("M117 phase37 bis");
  await page.getByTestId("custom-gcode-ok").click();
  await expect(page.getByTestId("editor-message")).toContainText("modifié");

  // Retranché : le code est accepté par le serveur et repéré à la couche 10.
  await sliceInUi(page);
  await expect(page.locator('[data-testid="layer-tick"][data-layer="9"]')).toHaveCount(1, {
    timeout: 60_000,
  });
  await expect(pending).toHaveCount(0);
});

test("« Changer le type » d'une pièce depuis le menu de l'objet @F:editor.volume-modifier @F:editor.volume-negative", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20"]);
  const row = page.getByTestId("object-row").first();
  await row.click({ button: "right" });
  await page.locator('[role=menu] [data-action="object.addModifier"]').click();
  const volume = page.getByTestId("object-volume").filter({ hasNot: page.getByTestId("object-volume-main") });
  await expect(volume).toHaveAttribute("data-type", "modifier");
  // Clic droit sur la pièce : section « Changer le type » en tête du menu de l'objet.
  await volume.click({ button: "right" });
  const section = page.locator('[role=menu] [data-testid="menu-section-section.volumeType"]');
  await expect(section).toBeVisible();
  await expect(section.getByRole("menuitem")).toHaveText([
    "Pièce",
    "Volume négatif",
    "Bloqueur de supports",
    "Forceur de supports",
  ]);
  await section.getByRole("menuitem", { name: "Volume négatif" }).click();
  await expect(volume).toHaveAttribute("data-type", "negative");
  await expect(page.getByTestId("editor-message")).toContainText("Modificateur → Volume négatif");
});

test("Tableau des paramètres d'objets : édition en grille, remise au profil @F:editor.object-settings @F:ui.object-settings", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20", "cylinder"]);
  await page.getByTestId("open-object-table").click();
  const table = page.getByTestId("object-table");
  await expect(table).toBeVisible();
  const rows = table.getByTestId("object-table-row");
  await expect(rows).toHaveCount(2);
  const cell = rows.first().locator('[data-testid=object-table-cell][data-key="wall_loops"]');
  await cell.fill("5");
  await cell.press("Enter");
  await expect(rows.first().getByTestId("object-table-reset")).toHaveCount(1);
  // Saisie invalide refusée (boucles entières).
  await cell.fill("2.5");
  await cell.press("Enter");
  await expect(table.getByTestId("object-table-error")).toBeVisible();
  // Catégorie « Vitesse » : colonnes de qualité masquées.
  await table.getByTestId("object-table-category").selectOption("speed");
  await expect(rows.first().locator('[data-col="wall_loops"]')).toHaveCount(0);
  await table.getByTestId("object-table-category").selectOption("all");
  await table.getByRole("button", { name: "Fermer", exact: true }).click();
  // Liste d'objets : 1 réglage surchargé sur le premier objet.
  await expect(page.getByTestId("object-overrides").first()).toHaveAttribute("data-settings", "1");
  // Remise au profil depuis le menu de l'objet (« Modifier dans la Table des Paramètres »).
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Modifier dans la Table des Paramètres" }).click();
  await table.getByTestId("object-table-reset").first().click();
  await expect(table.getByTestId("object-table-reset")).toHaveCount(0);
});

test("« Fusionner avec » un filament : objets réaffectés, slot retiré @F:editor.filament-slots", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20"]);
  await page.getByTestId("add-filament-slot").click();
  await page.getByTestId("add-filament-slot").click();
  const slots = page.getByTestId("filament-slot");
  await expect(slots).toHaveCount(3);
  const extruder = page.getByTestId("object-row").first().locator('select[aria-label^="Extrudeur de"]');
  await extruder.selectOption("3");
  await slots.nth(2).click({ button: "right" });
  await page.getByTestId("menu-sub-filament.mergeMenu").click();
  await page.getByRole("menuitem", { name: /^Filament 1/ }).click();
  await expect(slots).toHaveCount(2);
  await expect(extruder).toHaveValue("1");
  await expect(page.getByTestId("editor-message")).toContainText("Filament 3 fusionné avec le filament 1");
});

test("Plateau déplacé en premier ; navigateur 3D (faces) et axes masquables @F:toolbar.addplate @F:menu.default-view @F:menu.top @F:editor.menu-view-prefs", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await ready(page, ["cube20"]);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByRole("button", { name: "Ajouter un plateau" }).first().click();
  const tabs = page.getByTestId("plate-tab");
  await expect(tabs).toHaveCount(2);
  const second = await tabs.nth(1).getAttribute("data-plate-id");
  // Le premier plateau n'a pas l'icône ; le second oui.
  await expect(tabs.first().getByTestId("plate-move-front")).toHaveCount(0);
  await tabs.nth(1).getByTestId("plate-move-front").click();
  await expect(tabs.first()).toHaveAttribute("data-plate-id", second!);
  await expect(tabs.first()).toHaveAttribute("data-active", "true");

  // Navigateur 3D : affiché par défaut (Orca show_3d_navigator), faces cliquables.
  const cube = page.getByTestId("view-cube");
  await expect(cube).toBeVisible();
  await cube.locator('[data-testid=view-cube-face][data-view="top"]').click({ force: true });
  await expect(cube).toHaveAttribute("data-facing", "top", { timeout: 10_000 });
  // Vue de dessus : la face avant est de chant ; retour à la vue par défaut
  // (Iso), où la face avant est visible et cliquable.
  await page.getByTestId("view-cube-home").click();
  await expect(cube).not.toHaveAttribute("data-facing", "top", { timeout: 10_000 });
  await cube.locator('[data-testid=view-cube-face][data-view="front"]').click();
  await expect(cube).toHaveAttribute("data-facing", "front", { timeout: 10_000 });
  // Préférences › Graphismes : masquer le navigateur et les axes.
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  await prefs.getByTestId("pref-tab-graphics").click();
  await prefs.getByTestId("pref-3d-navigator").uncheck();
  await expect(prefs.getByTestId("pref-show-axes")).toBeChecked();
  await prefs.getByTestId("pref-show-axes").uncheck();
  await page.getByTestId("preferences-close").click();
  await expect(cube).toHaveCount(0);
});

test("Débit de buse Standard / Haut débit par extrudeur (H2D) @F:core.settings-ui", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const res = await page.request.post("/api/_harness/bootstrap", {
    data: {
      email: uniqueEmail(),
      password: "harness-password-123",
      printer: "Bambu Lab H2D 0.4 nozzle",
      filament: "Bambu PLA Basic @BBL H2D",
      process: "0.20mm Standard @BBL H2D",
    },
  });
  test.skip(res.status() !== 200, `presets H2D indisponibles (${res.status()})`);
  const { project_id } = (await res.json()) as { project_id: string };
  await page.goto(`/projects/${project_id}`);
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  const selects = page.getByTestId("nozzle-flow-select");
  await expect(selects).toHaveCount(2, { timeout: 30_000 });
  await expect(selects.first().locator("option")).toHaveText(["Standard", "Haut débit"]);
  await selects.first().selectOption("High Flow");
  await expect(selects.first()).toHaveValue("High Flow");
  await expect(page.getByTestId("editor-message")).toContainText("Extrudeur 1 : débit Haut débit");
});

test("Bambu-less : pas de sélecteur de débit sur une Voron @F:core.settings-ui", async ({ page }) => {
  await ready(page, []);
  await expect(page.getByTestId("nozzle-flow")).toHaveCount(0);
});

test("Informations de coupe : pièces liées, sélection groupée, « Invalider » (serveur) @F:gizmo.cut @F:editor.mesh-cut", async ({
  page,
}) => {
  test.setTimeout(150_000);
  const projectId = await ready(page, ["cube20", "cylinder"]);
  await saveNow(page);
  // Deux pièces d'une même coupe (cut_id d'Orca), posées dans la scène serveur.
  const project = await (await page.request.get(`/api/projects/${projectId}`)).json();
  const scene = project.scene as { objects: { cutId?: unknown }[] };
  for (const o of scene.objects) o.cutId = { id: 4, checkSum: 2, connectorsCnt: 1 };
  const put = await page.request.put(`/api/projects/${projectId}`, {
    data: { expected_version: Number(project.version), scene, active_presets: project.active_presets },
  });
  expect(put.status()).toBeLessThan(300);
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 60_000 });
  await expect(page.getByTestId("object-cut-info")).toHaveCount(2);
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Sélectionner les pièces liées" }).click();
  await expect(page.getByTestId("editor-message")).toContainText("2 pièce(s) de la même coupe");
  await page.getByTestId("object-row").first().click({ button: "right" });
  const [inv] = await Promise.all([
    page.waitForResponse((r) => /\/invalidate-cut$/.test(r.url())),
    page.getByRole("menuitem", { name: "Invalider les informations de découpe" }).click(),
  ]);
  expect(inv.status()).toBe(200);
  await expect(page.getByTestId("object-cut-info")).toHaveCount(0);
  await expect(page.getByTestId("editor-message")).toContainText("retirées de 2 pièce(s)");
  // Persisté côté serveur : plus de cutId.
  const after = await (await page.request.get(`/api/projects/${projectId}`)).json();
  expect((after.scene.objects as { cutId?: unknown }[]).every((o) => !o.cutId)).toBe(true);
});

test("Erreurs de maillage : icône d'alerte et infobulle d'Orca, clic → réparer @F:editor.mesh-repair @F:ctx.fix-model", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await bootstrap(page);
  await page.getByTestId("import-input").setInputFiles({
    name: "holed.stl",
    mimeType: "model/stl",
    buffer: brokenCube(),
  });
  await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await saveNow(page);
  const warning = page.getByTestId("object-mesh-warning");
  await expect(warning).toBeVisible({ timeout: 30_000 });
  await expect(warning).toHaveAttribute("data-open-edges", "3");
  await expect(warning).toHaveAttribute("data-tip", /3 arêtes non-manifold/);
  await expect(warning).toHaveAttribute("data-tip", /Cliquer sur l’icône pour réparer/);
  const [repair] = await Promise.all([
    page.waitForResponse((r) => /\/api\/models\/[^/]+\/repair$/.test(r.url()), { timeout: 60_000 }),
    warning.click(),
  ]);
  expect(repair.status()).toBe(200);
  await expect(warning).toHaveCount(0);
});

test("Qualité Draco des préférences transmise à l'export DRC @F:ctx.export-as-one-drc @F:menu.preferences", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page, ["cube20"]);
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  await prefs.getByTestId("pref-tab-behaviour").click();
  const bits = prefs.getByTestId("pref-draco-bits");
  await bits.fill("5");
  await bits.press("Tab");
  // Bornes d'Orca (create_item_draco) : 1–7 → 8.
  await expect(bits).toHaveValue("8");
  await page.getByTestId("preferences-close").click();
  await page.getByTestId("object-row").first().click({ button: "right" });
  const [req] = await Promise.all([
    page.waitForRequest((r) => r.method() === "POST" && /\/export\/drc$/.test(r.url()), {
      timeout: 60_000,
    }),
    page.locator('[role=menu] [data-action="object.exportDrc"]').click(),
  ]);
  expect(req.postDataJSON()).toMatchObject({ bits: 8, split: false });
});
