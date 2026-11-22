// Phase 48 — corrections de l'audit 46 (navigateur), partie interface :
// hauteur de couche variable sur toute la hauteur de l'objet (VLH-a), glisser au
// bouton droit sans menu contextuel (PREF-c), Réglages du plateau défilants en
// 800 × 500 (PLATE-a), surplombs de la peinture de supports (SUPP-a), refus de
// plaque traduit (I18N-a), imprimantes Bambu : export carte SD proposé (BBL-a).
//
// Oracle Orca 2.4.1 : GLCanvas3D::LayersEditing (m_object_max_z),
// PrintObject::update_layer_height_profile, GLCanvas3D.cpp:4599/4625/4726
// (ignore_right_up), PlateSettingsDialog (OtherLayersSeqPanel = wxScrolledWindow),
// GLGizmoFdmSupports.cpp:84-86, 381-420, 512-526, Print.cpp:1771 +
// OrcaSlicer_fr.po:13037.
//
// CONTRAT: svg[role=slider][aria-valuemax], plate-settings-body, settings-apply,
// paint-overhang, paint-overhang-only, paint-overhang-angle, paint-overhang-angle-input,
// scene-canvas[data-overhang-faces], slice-error, send-connection-help,
// send-connection-hint, send-export-sliced, send-moonraker-url.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel, uniqueEmail } from "../lib/app";

type Obj = { name?: string; layerProfile?: number[] };
type SceneDoc = { objects: Obj[] };

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  return id;
}
async function bootstrapWith(
  page: Page,
  printer: string,
  filament: string,
  process: string,
) {
  const res = await page.request.post("/api/_harness/bootstrap", {
    data: {
      email: uniqueEmail(),
      password: "harness-password-123",
      printer,
      filament,
      process,
    },
  });
  expect(res.status(), await res.text()).toBe(200);
  const { project_id } = (await res.json()) as { project_id: string };
  await page.goto("/projects/" + project_id);
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  return project_id;
}
async function scene(page: Page, id: string): Promise<SceneDoc> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return ((await res.json()) as { scene: SceneDoc }).scene;
}
async function saveProject(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status()).toBeLessThan(300);
}
async function selectFirst(page: Page) {
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("object-name").first().click();
}
const pairs = (flat: number[]) => {
  const out: [number, number][] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push([flat[i], flat[i + 1]]);
  return out;
};

test.describe("audit 46 (FR)", () => {
  test("VLH-a : la barre de hauteur de couche couvre tout l'objet ; un trait de pinceau modifie le profil adaptatif sans l'effacer ; le tranchage suit le profil @F:toolbar.layersediting", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    const id = await ready(page, ["cube20"]);
    await selectFirst(page);
    await page.getByTestId("toolbar-layersediting").click();
    const bar = page.locator("svg[role=slider]");
    // Cube de 20 mm (maillage recentré −10…10) : 20, pas le repli de 10.
    await expect(bar).toHaveAttribute(
      "aria-valuemax",
      /^(19\.9\d*|20(\.0\d*)?)$/,
    );

    await Promise.all([
      page.waitForResponse(
        (r) =>
          /\/layer-profile$/.test(r.url()) && r.request().method() === "POST",
      ),
      page.getByRole("button", { name: "Adaptatif", exact: true }).click(),
    ]);
    await saveProject(page);
    const adaptive = (await scene(page, id)).objects[0].layerProfile ?? [];
    expect(adaptive.length).toBeGreaterThan(2);
    expect(adaptive[adaptive.length - 2]).toBeCloseTo(20, 2);

    // Trait clic gauche (affiner) au milieu de la barre.
    const box = (await bar.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.waitForTimeout(700);
    await page.mouse.up();
    await expect(page.getByRole("menu")).toHaveCount(0);
    let brushed: number[] = [];
    await expect
      .poll(
        async () => {
          await saveProject(page);
          brushed = (await scene(page, id)).objects[0].layerProfile ?? [];
          return JSON.stringify(brushed) !== JSON.stringify(adaptive);
        },
        { timeout: 30_000 },
      )
      .toBe(true);
    // Profil couvrant tout l'objet (sinon update_layer_height_profile l'efface).
    expect(brushed[brushed.length - 2]).toBeCloseTo(20, 2);
    const heights = pairs(brushed);
    const adaptiveMax = Math.max(...pairs(adaptive).map(([, h]) => h));
    // Loin du trait (bas de l'objet), le profil adaptatif est conservé…
    const adaptivePairs = pairs(adaptive);
    const adaptiveAt = (z: number) => {
      for (let i = 1; i < adaptivePairs.length; i++) {
        const [z0, h0] = adaptivePairs[i - 1];
        const [z1, h1] = adaptivePairs[i];
        if (z <= z1)
          return z1 === z0 ? h1 : h0 + ((h1 - h0) * (z - z0)) / (z1 - z0);
      }
      return adaptivePairs[adaptivePairs.length - 1][1];
    };
    const low = heights.filter(([z]) => z > 0.5 && z < 4);
    expect(low.length).toBeGreaterThan(0);
    for (const [z, h] of low) expect(h).toBeCloseTo(adaptiveAt(z), 3);
    // … et au milieu, les couches sont plus fines.
    expect(
      Math.min(...heights.filter(([z]) => z > 8 && z < 12).map(([, h]) => h)),
    ).toBeLessThan(adaptiveMax);

    // Clic droit (épaissir) sur la barre : jamais le menu de la vue.
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.8);
    await page.mouse.down({ button: "right" });
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.85, {
      steps: 4,
    });
    await page.mouse.up({ button: "right" });
    await expect(page.getByRole("menu")).toHaveCount(0);

    await page.getByTestId("slice-button").click();
    const preview = page.getByTestId("preview-view");
    await expect(preview).toHaveAttribute("data-layer-count", /^[1-9]\d*$/, {
      timeout: 170_000,
    });
    // 0,2 mm uniformes = 100 couches : le profil a bien été pris en compte.
    expect(Number(await preview.getAttribute("data-layer-count"))).not.toBe(
      100,
    );
  });

  test("PREF-c : glisser au bouton droit dans la vue n'ouvre pas le menu contextuel ; un simple clic droit l'ouvre @F:menu.preferences", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await ready(page, ["cube20"]);
    const canvas = page.getByTestId("scene-canvas");
    const box = (await canvas.boundingBox())!;
    const x = box.x + box.width * 0.82;
    const y = box.y + box.height * 0.82;
    await page.mouse.move(x, y);
    await page.mouse.down({ button: "right" });
    await page.mouse.move(x - 120, y - 30, { steps: 8 });
    await page.mouse.up({ button: "right" });
    await page.waitForTimeout(300);
    await expect(page.getByRole("menu")).toHaveCount(0);
    // Clic droit sans déplacement : menu (comme Orca).
    await page.mouse.click(x - 120, y - 30, { button: "right" });
    await expect(page.getByRole("menu")).toBeVisible();
    await page.keyboard.press("Escape");
  });

  test("PLATE-a : 800 × 500, 3 filaments, séquences personnalisées et 3 plages — corps défilant, titre et Enregistrer visibles @F:editor.plate-settings", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await page.setViewportSize({ width: 800, height: 500 });
    await ready(page, ["cube20"]);
    await page.getByTestId("add-filament-slot").click();
    await page.getByTestId("add-filament-slot").click();
    await page.getByTestId("plate-settings-button").first().click();
    const dialog = page.getByTestId("settings-dialog");
    await dialog.getByTestId("settings-mode-develop").click();
    await dialog
      .locator("[data-param=first_layer_sequence_choice] select")
      .selectOption("Customize");
    await dialog
      .locator("[data-param=other_layers_sequence_choice] select")
      .selectOption("Customize");
    await dialog.getByTestId("plate-layer-range-add").click();
    await dialog.getByTestId("plate-layer-range-add").click();
    await expect(dialog.getByTestId("plate-layer-range")).toHaveCount(3);
    const d = (await dialog.boundingBox())!;
    expect(d.y).toBeGreaterThanOrEqual(0);
    expect(d.y + d.height).toBeLessThanOrEqual(500);
    await expect(
      dialog.getByRole("heading", { name: "Réglages du plateau" }),
    ).toBeInViewport();
    const apply = dialog.getByTestId("settings-apply");
    await expect(apply).toBeInViewport();
    const body = dialog.getByTestId("plate-settings-body");
    const scrollable = await body.evaluate(
      (el) => el.scrollHeight > el.clientHeight + 1,
    );
    expect(scrollable).toBe(true);
    // La dernière plage est atteignable par défilement.
    await dialog
      .getByTestId("plate-layer-range-delete")
      .scrollIntoViewIfNeeded();
    await expect(
      dialog.getByTestId("plate-layer-range-delete"),
    ).toBeInViewport();
    // Orca ignore les plages jamais remplies (phase 51 : une ligne vide
    // n'invalide plus l'application) : on remplit les plages ajoutées.
    await dialog.getByTestId("plate-layer-range").nth(1).getByTestId("plate-layer-range-begin").fill("3");
    await dialog.getByTestId("plate-layer-range").nth(1).getByTestId("plate-layer-range-begin").press("Tab");
    await dialog.getByTestId("plate-layer-range").nth(2).getByTestId("plate-layer-range-begin").fill("5");
    await dialog.getByTestId("plate-layer-range").nth(2).getByTestId("plate-layer-range-begin").press("Tab");
    await apply.click();
    await expect(dialog).toHaveCount(0);
  });

  test("SUPP-a : peinture de supports — surplombs mis en évidence selon l'angle, peinture limitée aux surplombs (un remplissage hors surplomb ne peint rien) @F:gizmo.fdmsupports @F:editor.paint-supports", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    // Pont (dessous du tablier en surplomb) + cube (faces verticales : pas de
    // surplomb), disposés côte à côte ; le cube est peint.
    await ready(page, ["cube20", "bridge"]);
    await page.getByTestId("toolbar-arrange").click();
    await page.getByTestId("arrange-all").click();
    await expect(page.getByTestId("overlap-warning")).toHaveCount(0);
    await selectFirst(page);
    await page.getByTestId("tool-support-paint").click();
    const panel = page.getByTestId("paint-overhang");
    await expect(
      panel.getByText("Mettre en surbrillance les zones en surplomb"),
    ).toBeVisible();
    await expect(
      panel.getByText("Uniquement sur les surplombs mis en évidence"),
    ).toBeVisible();
    // Angle initial = seuil des supports auto du profil, 0 sans support auto
    // (rien en évidence).
    const initial = await page
      .getByTestId("paint-overhang-angle-input")
      .inputValue();
    const canvas = page.getByTestId("scene-canvas");
    if (initial === "0")
      await expect(canvas).not.toHaveAttribute("data-overhang-faces", /^[1-9]/);
    await page.getByTestId("paint-overhang-angle-input").fill("60");
    await page.getByTestId("paint-overhang-angle-input").press("Tab");
    // Dessous du tablier du pont : facettes en surplomb.
    await expect(canvas).toHaveAttribute("data-overhang-faces", /^[1-9]\d*$/);

    // « Seulement sur les surplombs en évidence » : le remplissage lancé depuis
    // une face verticale du cube (pas un surplomb) ne peint rien…
    await page.getByTestId("paint-overhang-only").check();
    await page.getByTestId("paint-tool-fill").click();
    await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
    const [ax, ay] = (await canvas.getAttribute("data-object-anchor"))!
      .split(",")
      .map(Number);
    const cb = (await canvas.boundingBox())!;
    await page.mouse.click(cb.x + ax, cb.y + ay);
    await page.waitForTimeout(2_000);
    await expect(page.getByText(/^0 triangle\(s\) peint\(s\)$/)).toBeVisible();
    // … et sans la restriction, le même clic peint (highlight_deg = 0).
    await page.getByTestId("paint-overhang-only").uncheck();
    await page.mouse.click(cb.x + ax, cb.y + ay);
    await expect(
      page.getByText(/^[1-9]\d* triangle\(s\) peint\(s\)$/),
    ).toBeVisible({ timeout: 20_000 });
    // Fermeture puis réouverture : angle remis au seuil des supports auto.
    await page.getByTestId("tool-support-paint").click();
    await expect(panel).toHaveCount(0);
    await page.getByTestId("tool-support-paint").click();
    await expect(page.getByTestId("paint-overhang-angle-input")).toHaveValue(
      initial,
    );
  });

  test("I18N-a : refus de plaque du tranchage traduit (« Plaque 1 : Cool Plate ne prend pas en charge le filament 1 ») @F:core.slice-ui", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await bootstrapWith(
      page,
      "Bambu Lab X1 Carbon 0.4 nozzle",
      "Bambu ABS @BBL X1C",
      "0.20mm Standard @BBL X1C",
    );
    await importModel(page, "cube20");
    await expect(page.getByTestId("import-status")).toHaveCount(0, {
      timeout: 60_000,
    });
    await page.getByTestId("slice-button").click();
    const error = page.getByTestId("slice-error");
    await expect(error).toBeVisible({ timeout: 170_000 });
    // Phase 49 (audit 47 I18N-b) : nom de plaque traduit comme Orca
    // (Print.cpp:1771 L(bed_type_name), OrcaSlicer_fr.po:10922).
    await expect(error).toContainText(
      "Plaque 1 : Cool Plate/Plaque PLA ne prend pas en charge le filament 1",
    );
    await expect(error).not.toContainText("does not support");
  });

  test("BBL-a : Bambu Lab — explication avant le champ d'adresse, export .gcode.3mf (carte SD) proposé @F:ui.print-hosts @F:menu.export-plate-sliced-file", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await bootstrapWith(
      page,
      "Bambu Lab P1S 0.4 nozzle",
      "Bambu PLA Basic @BBL X1C",
      "0.20mm Standard @BBL X1C",
    );
    await importModel(page, "cube20");
    await page.getByTestId("slice-button").click();
    await expect(page.getByTestId("preview-view")).toHaveAttribute(
      "data-layer-count",
      /^[1-9]\d*$/,
      {
        timeout: 170_000,
      },
    );
    await page.getByTestId("send-to-printer").click();
    const help = page.getByTestId("send-connection-help");
    await expect(help.getByTestId("send-connection-hint")).toContainText(
      "réseau Bambu",
    );
    const helpBox = (await help.boundingBox())!;
    const urlBox = (await page
      .getByTestId("send-moonraker-url")
      .boundingBox())!;
    expect(helpBox.y).toBeLessThan(urlBox.y);
    const [download] = await Promise.all([
      page.waitForEvent("download", { timeout: 120_000 }),
      help.getByTestId("send-export-sliced").click(),
    ]);
    expect(download.suggestedFilename()).toMatch(/\.gcode\.3mf$/);
  });
});
