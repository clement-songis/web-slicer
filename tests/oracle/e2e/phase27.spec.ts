// Phase 27 — frictions de la phase 26 et 3e audit : état de chargement à
// l'import, objets teintés au filament, nom d'objet jamais tronqué, onglets
// vides masqués en Simple, modificateur ajouté sélectionné, contrôle objet par
// objet avant tranchage, « Profil (…) » des plateaux, onglet « Forçage des
// réglages » sans effet sur le brouillon, import illisible sans renommage.
import { expect, test } from "@playwright/test";
import { bootstrap, fixture, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

test("import : état de chargement explicite puis objet teinté à la couleur du filament @F:core.import", async ({
  page,
}) => {
  await bootstrap(page);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.getByTestId("import-input").setInputFiles(fixture("cube20"));
  // Avant que l'objet n'apparaisse, la vue l'annonce au lieu d'inviter à glisser un modèle.
  await expect(page.getByTestId("import-loading")).toContainText("cube20.stl");
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("import-loading")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("scene-canvas")).toHaveAttribute(
    "data-extruder-colors",
    "1",
  );
  // Un seul filament : pas de sélecteur d'extrudeur (colonne masquée comme Orca).
  await expect(page.getByLabel(/^Extrudeur de /)).toHaveCount(0);
});

test("liste d’objets : un nom long reste entier même avec badges et sélecteurs @F:editor.rename", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  await page.getByTestId("add-filament-slot").click();
  await page
    .getByRole("button", { name: /^(Ajouter un|Nouveau) plateau$/ })
    .first()
    .click();
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("object-name").first().dblclick();
  const input = page.getByTestId("object-rename-input");
  await input.fill("support-de-telephone-articule-v3-final.stl");
  await input.press("Enter");
  const name = page.getByTestId("object-name").first();
  await expect(name).toHaveText(/support-de-telephone-articule-v3-final\.stl/);
  await expect(page.getByLabel(/^Extrudeur de /)).toHaveCount(1);
  await expect(page.getByLabel(/^Plateau de /)).toHaveCount(1);
  const overflow = await name.evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await expect(name).toHaveAttribute(
    "title",
    "support-de-telephone-articule-v3-final.stl",
  );
});

test("réglages : onglets vides masqués en Simple, lien vers le mode qui les montre @F:core.settings-ui", async ({
  page,
}) => {
  await bootstrap(page);
  await page.getByTestId("open-settings-printer").click();
  const dialog = page.getByTestId("settings-dialog");
  await dialog.getByTestId("settings-mode-simple").click();
  await expect(dialog.getByTestId("settings-tab-machine-g-code")).toHaveCount(
    0,
  );
  const more = dialog.getByTestId("settings-hidden-pages");
  await expect(more).toContainText("en mode Avancé");
  await more.click();
  await expect(dialog.getByTestId("settings-mode-advanced")).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(
    dialog.getByTestId("settings-tab-machine-g-code"),
  ).toHaveAttribute("aria-selected", "true");
  // Libellés des éditeurs de G-code traduits (OrcaSlicer_fr.po).
  const start = dialog.locator("[data-param=machine_start_gcode]");
  await expect(start).toContainText("G-code de démarrage");
  await expect(start).not.toContainText("Start G-code");
});

test("modificateur ajouté : sélectionné, ses réglages ouverts @F:editor.volume-transform", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page
    .getByRole("menuitem", { name: /Add modifier|Ajouter un modificateur/i })
    .click();
  await expect(page.getByTestId("scene-canvas")).toHaveAttribute(
    "data-selected-volume",
    /.+/,
  );
  await expect(
    page.getByTestId("object-volume-select").first(),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("volume-transform")).toBeVisible();
  // Un second clic garde la sélection (le nom de l'objet ramène à l'objet).
  await page.getByTestId("object-volume-select").first().click();
  await expect(
    page.getByTestId("object-volume-select").first(),
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByTestId("object-volume-main").click();
  await expect(page.getByTestId("scene-canvas")).not.toHaveAttribute(
    "data-selected-volume",
    /.+/,
  );
});

test("objet par objet : objets trop proches signalés avant tranchage, « Disposer » corrige @F:editor.plate-settings", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  // Phase 28 : le 2e import est posé à côté (Orca) — plus de superposition à disposer.
  await expect(page.getByTestId("overlap-warning")).toHaveCount(0, {
    timeout: 30_000,
  });
  // « Profil (…) » : valeur suivie quand le plateau ne surcharge rien.
  const seq = page.getByTestId("plate-setting-print_sequence");
  await expect(seq.locator("option").first()).toHaveText(/^Profil \(.+\)$/);
  await seq.selectOption("by object");
  // Phase 29 : l'estimation locale est remplacée par la validation serveur
  // (Print::validate) — message d'Orca traduit, « Disposer » dans la notification.
  const warning = page.locator(
    '[data-testid=validation-notice][data-kind=object_collision_in_seq_print]',
  );
  await expect(warning.first()).toContainText("trop proche des autres", {
    timeout: 60_000,
  });
  await expect(warning.first()).toContainText("cylinder.stl");
  await warning.first().getByTestId("validation-arrange").click();
  await expect(warning).toHaveCount(0, { timeout: 60_000 });
});

test("filament : afficher « Forçage des réglages » ne casse pas le tranchage @F:core.slice-ui", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByTestId("open-settings-filament").click();
  const dialog = page.getByTestId("settings-dialog");
  await dialog.getByTestId("settings-mode-develop").click();
  await dialog.getByTestId("settings-tab-setting-overrides").click();
  await expect(
    dialog.locator("[data-param=filament_retract_lift_above]"),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  const [req] = await Promise.all([
    page.waitForRequest(
      (r) => r.method() === "POST" && /\/slice$/.test(r.url()),
    ),
    page.getByTestId("slice-button").click(),
  ]);
  const body = JSON.parse(req.postData() ?? "{}") as {
    draft?: { filaments?: unknown[] };
  };
  expect(JSON.stringify(body.draft ?? {})).not.toMatch(
    /filament_retract|filament_wipe/,
  );
  await expect(page.getByTestId("export-gcode-button")).toBeEnabled({
    timeout: 180_000,
  });
});

test("import illisible : message nommant le fichier, projet non renommé @F:core.import", async ({
  page,
}) => {
  await bootstrap(page);
  await page.getByTestId("import-input").setInputFiles({
    name: "casse.stl",
    mimeType: "model/stl",
    buffer: Buffer.from("solid x\nfacet normal 0 0 1\n garbage\nendsolid"),
  });
  // Phase 29 : la cause du moteur (lot 27) remplace le message générique.
  await expect(
    page.getByText(/casse\.stl : conversion impossible — .+/),
  ).toBeVisible({
    timeout: 60_000,
  });
  await expect(page.getByTestId("object-row")).toHaveCount(0);
  await expect(page).not.toHaveTitle(/casse/);
});
