// Phase 38 — interface des outils de peinture du lot 38 (Triangle, Plage de
// hauteur, pot sans arêtes, aperçu au survol, remappage MMU, comblement,
// contrainte Verticale / Horizontale), vue réaliste, restes des phases 36
// (superposition avec rotation, unité €/kg, nom de la langue, événements du
// résumé, 3MF vide lisible, recherche de filament) et 37 (menu d'un objet
// coupé à 1280×720, navigateur 3D en Aperçu, fusion de filaments qui
// renumérote la peinture MMU et le filament de support).
import { expect, test, type Page, type Request } from "@playwright/test";
import { join } from "node:path";
import { ORACLE, bootstrap, importModel, paramRow, sliceInUi } from "../lib/app";

test.use({ locale: "fr-FR" });

type Msg = Record<string, any>;

/** Messages du WebSocket de peinture (envoyés / reçus). */
function paintSocket(page: Page) {
  const sent: Msg[] = [];
  const received: Msg[] = [];
  page.on("websocket", (ws) => {
    if (!/\/paint$/.test(ws.url())) return;
    ws.on("framesent", (f) => {
      try {
        sent.push(JSON.parse(String(f.payload)));
      } catch {
        /* binaire */
      }
    });
    ws.on("framereceived", (f) => {
      try {
        received.push(JSON.parse(String(f.payload)));
      } catch {
        /* binaire */
      }
    });
  });
  return { sent, received };
}

async function ready(page: Page, models = ["cube20"]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
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

async function anchor(page: Page): Promise<[number, number]> {
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
  const [x, y] = (await canvas.getAttribute("data-object-anchor"))!.split(",").map(Number);
  const box = (await canvas.boundingBox())!;
  return [box.x + x, box.y + y];
}

async function clickObject(page: Page, dx = 0, dy = 0) {
  const [x, y] = await anchor(page);
  await page.mouse.click(x + dx, y + dy);
}

const painted = (page: Page) => page.getByText(/^[1-9]\d* triangle\(s\) peint\(s\)$/);

/** « + » des filaments (onglet Traitement de la colonne), puis retour aux objets. */
async function addFilament(page: Page) {
  await page.getByRole("button", { name: "Traitement", exact: true }).click();
  await page.getByTestId("add-filament-slot").click();
  await page.getByRole("button", { name: "Objets", exact: true }).click();
}

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

/** « Effacer » du panneau de peinture : canal vidé (facettes {}). */
async function clearPaint(page: Page) {
  await page.getByTestId("paint-frame-selection").locator("..").getByRole("button", { name: "Effacer" }).click();
  await expect(painted(page)).toHaveCount(0);
}

// Chaque opération part d'un canal vide : le backend refuse aujourd'hui tout
// message dont `facets` n'est pas vide (voir le test « surface déjà peinte »).
test("peinture MMU : Triangle, Plage de hauteur, pot sans arêtes, aperçu, remappage, comblement @F:gizmo.mmsegmentation", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const ws = paintSocket(page);
  const id = await ready(page);
  await addFilament(page);
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-mm-paint").click();
  for (const t of ["brush", "sphere", "pointer", "height", "fill", "gap"])
    await expect(page.getByTestId(`paint-tool-${t}`)).toBeVisible();
  const replies = (type: string) => ws.received.filter((m) => m.type === type);

  // Triangle (cursor pointer) avec le filament 2.
  await page.getByTestId("paint-filament-2").click();
  await page.getByTestId("paint-tool-pointer").click();
  await expect(page.getByTestId("paint-constraint-vertical")).toBeVisible();
  await clickObject(page);
  await expect
    .poll(() => ws.sent.filter((m) => m.type === "stroke" && m.stroke?.cursor === "pointer").length)
    .toBeGreaterThan(0);
  await expect(painted(page)).toBeVisible();
  await expect.poll(() => replies("facets").length).toBe(1);
  await clearPaint(page);

  // Plage de hauteur : height envoyé, facettes renvoyées par le moteur.
  await page.getByTestId("paint-tool-height").click();
  await page.getByTestId("paint-height").fill("2");
  await clickObject(page, 4, 4);
  await expect
    .poll(() => ws.sent.find((m) => m.type === "stroke" && m.stroke?.cursor === "height_range")?.stroke.height)
    .toBe(2);
  await expect(painted(page)).toBeVisible();
  await expect.poll(() => replies("facets").length).toBe(2);
  await clearPaint(page);

  // Pot sans détection d'arêtes : aperçu au survol, puis clic (fill_angle = -1).
  await page.getByTestId("paint-tool-fill").click();
  await page.getByTestId("paint-edge-detection").uncheck();
  await expect(page.getByTestId("paint-fill-angle")).toHaveCount(0);
  const [x, y] = await anchor(page);
  await page.mouse.move(x - 3, y - 3);
  await page.mouse.move(x, y, { steps: 4 });
  await expect
    .poll(() => ws.sent.filter((m) => m.type === "preview" && m.stroke?.tool === "bucket_fill").length)
    .toBeGreaterThan(0);
  expect(
    ws.sent.find((m) => m.type === "preview" && m.stroke?.tool === "bucket_fill")!.stroke.fill_angle,
  ).toBe(-1);
  await expect(page.getByTestId("scene-canvas")).toHaveAttribute("data-hover-preview", "true");
  await page.mouse.click(x, y);
  await expect
    .poll(() => ws.sent.find((m) => m.type === "stroke" && m.stroke?.tool === "bucket_fill")?.stroke.fill_angle)
    .toBe(-1);
  await expect(painted(page)).toBeVisible();
  await expect.poll(() => replies("facets").length).toBe(3);
  await clearPaint(page);

  // Remappage (table source → destination) : filament de base de l'objet
  // (1 → 2) ; sans peinture, aucun message moteur.
  await page.getByTestId("paint-remap-open").click();
  await expect(page.getByTestId("paint-remap-apply")).toBeDisabled();
  await page.getByTestId("paint-remap-1").selectOption("1");
  await page.getByTestId("paint-remap-apply").click();
  await expect(page.getByTestId("editor-message")).toContainText("Filaments remappés (filament de base)");
  await expect(
    page.getByTestId("object-row").first().locator('select[aria-label^="Extrudeur de"]'),
  ).toHaveValue("2");
  expect(ws.sent.filter((m) => m.type === "remap")).toHaveLength(0);

  // Comblement : bouton « Combler » (aire 2 mm²), sans clic sur l'objet.
  await page.getByTestId("paint-tool-gap").click();
  await expect(page.getByTestId("paint-gap-apply")).toBeDisabled();
  await page.getByTestId("paint-gap-area").fill("2");
  await page.getByTestId("paint-gap-apply").click();
  await expect.poll(() => ws.sent.find((m) => m.type === "gap_fill")?.area).toBe(2);
  await expect(page.getByTestId("editor-message")).toContainText("Comblement");
  expect(replies("error").map((m) => m.message)).toEqual([]);
  expect(replies("preview").length).toBeGreaterThan(0);
  await saveNow(page);
  const project = await (await page.request.get(`/api/projects/${id}`)).json();
  expect(project.scene.objects[0]).toBeTruthy();
});

// Régression : le serveur refusait toute facette non vide (clés JSON chaînes
// lues comme u32 dans un enum serde étiqueté) → second trait, remappage,
// comblement et aperçu échouaient sur un canal déjà peint.
test("peinture : second trait sur une surface déjà peinte et remappage de la peinture @F:gizmo.mmsegmentation", async ({
  page,
}) => {
  test.setTimeout(150_000);
  const ws = paintSocket(page);
  await ready(page);
  await addFilament(page);
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-mm-paint").click();
  await page.getByTestId("paint-tool-pointer").click();
  await clickObject(page);
  await expect(painted(page)).toBeVisible();
  await clickObject(page, 5, 5);
  await expect.poll(() => ws.sent.filter((m) => m.type === "stroke").length).toBe(2);
  await page.getByTestId("paint-remap-open").click();
  await page.getByTestId("paint-remap-1").selectOption("1");
  await page.getByTestId("paint-remap-apply").click();
  await expect.poll(() => ws.sent.find((m) => m.type === "remap")?.map).toEqual([2, 2]);
  await expect.poll(() => ws.received.filter((m) => m.type === "facets").length, { timeout: 20_000 }).toBe(3);
  expect(ws.received.filter((m) => m.type === "error")).toEqual([]);
});

test("supports : outils d'Orca, Remplissage = smart fill ; couture : contrainte Verticale @F:gizmo.fdmsupports @F:gizmo.seam @F:gizmo.fuzzyskin", async ({
  page,
}) => {
  test.setTimeout(150_000);
  const ws = paintSocket(page);
  await ready(page);
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-support-paint").click();
  for (const t of ["brush", "sphere", "fill", "gap"])
    await expect(page.getByTestId(`paint-tool-${t}`)).toBeVisible();
  await expect(page.getByTestId("paint-tool-pointer")).toHaveCount(0);
  await expect(page.getByTestId("paint-tool-height")).toHaveCount(0);
  await page.getByTestId("paint-tool-fill").click();
  await expect(page.getByTestId("paint-fill-angle")).toBeVisible();
  await clickObject(page);
  await expect
    .poll(() => ws.sent.find((m) => m.type === "stroke" && m.channel === "supports")?.stroke.tool)
    .toBe("smart_fill");

  // Fuzzy skin : outil Triangle (pas de plage de hauteur).
  await page.getByTestId("tool-fuzzy-paint").click();
  await expect(page.getByTestId("paint-tool-pointer")).toBeVisible();
  await expect(page.getByTestId("paint-tool-height")).toHaveCount(0);

  // Couture : « Verticale » seule (GLGizmoSeam) ; un glisser horizontal contraint
  // reste sur le premier point.
  await page.getByTestId("tool-seam-paint").click();
  await page.getByTestId("paint-tool-brush").click();
  await expect(page.getByTestId("paint-constraint-horizontal")).toHaveCount(0);
  await page.getByTestId("paint-constraint-vertical").check();
  const drag = async () => {
    const [x, y] = await anchor(page);
    await page.mouse.move(x - 12, y);
    await page.mouse.down();
    await page.mouse.move(x + 12, y, { steps: 8 });
    await page.mouse.up();
  };
  const seamStrokes = () => ws.sent.filter((m) => m.type === "stroke" && m.channel === "seam");
  await drag();
  await expect.poll(() => seamStrokes().length).toBe(1);
  expect(seamStrokes()[0].stroke.points).toHaveLength(1);
  await page.getByTestId("paint-constraint-vertical").uncheck();
  await drag();
  await expect.poll(() => seamStrokes().length).toBe(2);
  expect(seamStrokes()[1].stroke.points.length).toBeGreaterThan(1);
});

test("vue réaliste : menu Vue et préférences, suspendue pendant la peinture des supports @F:editor.menu-view-prefs @F:menu.preferences", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page);
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-realistic", "off");
  await page.getByRole("button", { name: /^(view|vue|affichage)$/i }).first().click();
  await page.getByRole("menuitem", { name: /^Vue réaliste$/ }).click();
  await expect(canvas).toHaveAttribute("data-realistic", "realistic phong");
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  await prefs.getByTestId("pref-tab-graphics").click();
  await expect(prefs.getByTestId("pref-realistic-view")).toBeChecked();
  await expect(prefs.getByTestId("pref-realistic-phong")).toBeChecked();
  await prefs.getByTestId("pref-realistic-shadows").check();
  await prefs.getByTestId("pref-realistic-smooth").check();
  await prefs.getByTestId("pref-realistic-phong").uncheck();
  await page.getByTestId("preferences-close").click();
  await expect(canvas).toHaveAttribute("data-realistic", "realistic gouraud shadows smooth");
  // Orca : vue réaliste coupée pendant Supports / Couture / Fuzzy, gardée en MMU.
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-support-paint").click();
  await expect(canvas).toHaveAttribute("data-realistic", "off");
  await page.getByTestId("tool-mm-paint").click();
  await expect(canvas).toHaveAttribute("data-realistic", /^realistic/);
});

test("restes phase 36 : langue nommée, €/kg, 3MF vide lisible, recherche de filament @F:menu.preferences @F:core.settings-ui @F:menu.open-project @F:editor.filament-slots", async ({
  page,
}) => {
  test.setTimeout(150_000);
  const id = await bootstrap(page);
  // Langue : nom de la langue, pas son code.
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  const lang = prefs.getByTestId("pref-language");
  await expect(lang.locator("option")).toContainText(["English", "Français"]);
  await expect(lang.locator("option:checked")).toHaveText("Français");
  await page.getByTestId("preferences-close").click();

  // Coût du filament : devise des préférences (€/kg), plus « argent/kg ».
  const row = await paramRow(page, "filament", "Filament", "filament_cost");
  await expect(row).toBeVisible();
  await expect(row).toContainText("€/kg");
  await expect(row).not.toContainText("argent");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);

  // Recherche dans la longue liste des filaments.
  const select = page.getByTestId("preset-select-filament");
  // Liste chargée de façon asynchrone (≈ 280 profils système).
  await expect.poll(() => select.locator("option").count(), { timeout: 30_000 }).toBeGreaterThan(30);
  const total = await select.locator("option").count();
  await page.getByTestId("preset-select-filament-search-toggle").click();
  await page.getByTestId("preset-select-filament-search").fill("PETG HF");
  await expect.poll(() => select.locator("option").count()).toBeLessThan(total / 4);
  await expect(select.locator("option", { hasText: /PETG HF/ }).first()).toBeAttached();
  await page.getByTestId("preset-select-filament-search").press("Escape");
  await expect(page.getByTestId("preset-select-filament-search")).toHaveCount(0);
  await expect.poll(() => select.locator("option").count()).toBe(total);

  // 3MF sans objet : message lisible, sans la sortie du worker.
  const ex = await page.request.get(`/api/projects/${id}/export/3mf`);
  const buffer = Buffer.from(await ex.body());
  await page.getByTestId("project-open-input").setInputFiles({ name: "vide.3mf", mimeType: "model/3mf", buffer });
  const message = page.getByTestId("editor-message");
  await expect(message).toContainText("ne contient aucun objet", { timeout: 30_000 });
  await expect(message).not.toContainText("worker");
  await expect(message).not.toContainText("stdout");
});

test("superposition : empreinte tournée, plus de faux positif ; G-code externe : tronçons du résumé @F:toolbar.arrange @F:editor.menu-open-gcode @F:preview.engine", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const id = await bootstrap(page);
  for (let i = 0; i < 5; i++) await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await saveNow(page);
  const project = await (await page.request.get(`/api/projects/${id}`)).json();
  const objects = project.scene.objects as Record<string, any>[];
  expect(objects).toHaveLength(5);
  // Transformation de l'objet du document scène (position / rotation en degrés / échelle).
  const place = (o: Record<string, any>, x: number, rz: number, scale = [1, 1, 1]) => {
    o.transform = { position: [x, 100, 10 * scale[2]], rotation: [0, 0, rz], scale };
  };
  // Barre (cube mis à l'échelle 2 × 0,5 × 0,5 = 40 × 10 × 10) tournée de 90° (empreinte 10 × 40) à 2 mm d'un cube : les
  // boîtes sans rotation (avant) les disaient superposés de 13 mm → plus d'alerte.
  place(objects[0], 60, 90, [2, 0.5, 0.5]);
  place(objects[1], 77, 0);
  // Cube tourné de 45° (losange de 28,3 mm) entre deux cubes à 22 mm : vrais
  // recouvrements (2,1 mm) que les boîtes sans rotation ne voyaient pas.
  place(objects[2], 160, 45);
  place(objects[3], 182, 0);
  place(objects[4], 138, 0);
  const put = await page.request.put(`/api/projects/${id}`, {
    data: { expected_version: Number(project.version), scene: project.scene, active_presets: project.active_presets },
  });
  expect(put.status()).toBeLessThan(300);
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(5, { timeout: 60_000 });
  await expect(page.getByTestId("scene-canvas")).toHaveAttribute(
    "data-object-positions",
    /:60\.000,100\.000,.*:182\.000,100\.000,/,
  );
  // Avant : « 2 objet(s) » (la barre et son voisin) ; maintenant les 3 cubes du losange.
  await expect(page.getByTestId("overlap-warning")).toContainText("3 objet(s) superposé(s)", { timeout: 60_000 });

  // G-code externe (pause + G-code perso) : Impression / Pause / Impression,
  // pas de « Changement de couleur » fantôme (tronçon final d'Orca).
  await page.getByTestId("gcode-open-input").setInputFiles(
    join(ORACLE, "../../engine/tests/fixtures/custom_gcode/cube_custom.orca.gcode"),
  );
  await expect(page.getByTestId("engine-summary")).toHaveAttribute("data-source", "engine", { timeout: 60_000 });
  const table = page.getByTestId("engine-partial-times");
  await expect(table).toBeVisible();
  // Cas de la phase 36 : 4m 50s d'impression (reste 9m 55s), pause, 5m 5s.
  await expect(table.locator("tbody tr")).toHaveText([
    /^\s*Impression\s*9m 55s\s*4m 50s$/,
    /^\s*Pause\s*$/,
    /^\s*Impression\s*5m 5s\s*5m 5s$/,
  ]);
  await expect(table).not.toContainText("Changement de couleur");
});

test("restes phase 37 : menu d'un objet coupé à 1280×720, navigateur 3D en Aperçu, fusion qui renumérote la peinture @F:gizmo.cut @F:core.preview @F:editor.filament-slots @F:gizmo.mmsegmentation", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await page.setViewportSize({ width: 1280, height: 720 });
  const id = await ready(page, ["cube20", "cylinder"]);
  await saveNow(page);
  const project = await (await page.request.get(`/api/projects/${id}`)).json();
  for (const o of project.scene.objects) o.cutId = { id: 4, checkSum: 2, connectorsCnt: 1 };
  const put = await page.request.put(`/api/projects/${id}`, {
    data: { expected_version: Number(project.version), scene: project.scene, active_presets: project.active_presets },
  });
  expect(put.status()).toBeLessThan(300);
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 60_000 });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  // Menu de la dernière ligne : entièrement dans la fenêtre, sans défilement.
  await page.getByTestId("object-row").last().click({ button: "right" });
  const menu = page.getByRole("menu").last();
  await expect(menu.getByRole("menuitem", { name: "Invalider les informations de découpe" })).toBeVisible();
  const m = await menu.evaluate((el) => {
    const r = el.getBoundingClientRect();
    return { top: r.top, bottom: r.bottom, scroll: el.scrollHeight - el.clientHeight };
  });
  expect(m.top).toBeGreaterThanOrEqual(0);
  expect(m.bottom).toBeLessThanOrEqual(720);
  expect(m.scroll).toBeLessThanOrEqual(1);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1600, height: 1000 });

  // Fusion : peinture MMU du filament 2 renumérotée vers le filament 1.
  await addFilament(page);
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-mm-paint").click();
  await page.getByTestId("paint-filament-2").click();
  await page.getByTestId("paint-tool-pointer").click();
  await clickObject(page);
  await expect(painted(page)).toBeVisible();
  await page.waitForTimeout(1500);
  await page.getByTestId("tool-mm-paint").click();
  await page.getByRole("button", { name: "Traitement", exact: true }).click();
  const slots = page.getByTestId("filament-slot");
  await slots.nth(1).click({ button: "right" });
  await page.getByTestId("menu-sub-filament.mergeMenu").click();
  await page.getByRole("menuitem", { name: /^Filament 1/ }).click();
  await expect(slots).toHaveCount(1);
  await expect(page.getByTestId("editor-message")).toContainText("peinture");
  await saveNow(page);
  const saved = await (await page.request.get(`/api/projects/${id}`)).json();
  const mmu = Object.values(saved.scene.objects[0].painting?.mmu ?? {}) as string[];
  expect(mmu.length).toBeGreaterThan(0);
  // Plus aucune feuille à l'état 2 (« 8 » en quartet Orca) : seulement l'état 1 (« 4 »).
  expect(mmu.every((hex) => !/8/.test(hex))).toBe(true);

  // Navigateur 3D aussi en Aperçu.
  await sliceInUi(page);
  const cube = page.getByTestId("preview-view-cube").getByTestId("view-cube");
  await expect(cube).toBeVisible();
  await cube.locator('[data-testid=view-cube-face][data-view="top"]').click({ force: true });
  await expect(cube).toHaveAttribute("data-facing", "top", { timeout: 10_000 });
});
