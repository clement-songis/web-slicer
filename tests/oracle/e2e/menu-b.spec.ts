// Lot menu-b : parité des menus principaux d'OrcaSlicer (MainFrame::init_menubar_as_editor
// et init_menubar_as_gcodeviewer). Chaque test vérifie un effet observable : état de
// scène persisté, préférence persistée, fichier téléchargé, projet / presets dérivés.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import {
  REPO,
  bootstrap,
  downloadGcode,
  importModel,
  sliceInUi,
} from "../lib/app";

const GCODE = join(REPO, "engine/tests/fixtures/cube_bbl.gcode");
const FILE = /^(file|fichier)$/i;
const EDIT = /^(edit|édition)$/i;
const CALIB = /^calibration$/i;

async function menu(page: Page, top: RegExp, item: RegExp, sub?: RegExp) {
  await page.getByRole("button", { name: top }).first().click();
  if (sub)
    await page
      .getByRole("menu")
      .getByRole("button", { name: sub })
      .first()
      .click();
  await page.getByRole("menuitem", { name: item }).first().click();
}
const edit = (page: Page, item: RegExp) => menu(page, EDIT, item);

async function ready(page: Page) {
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

async function saveProject(page: Page, projectId: string) {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        new URL(r.url()).pathname === "/api/projects/" + projectId,
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status()).toBe(200);
  await expect(page.getByTestId("save-project")).toBeEnabled();
}

const selected = (page: Page) =>
  page.locator('[data-testid="object-row"].bg-primary');

function zipEntries(bytes: Buffer): { name: string; data: Buffer }[] {
  const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  const count = bytes.readUInt16LE(end + 10);
  let at = bytes.readUInt32LE(end + 16);
  const out: { name: string; data: Buffer }[] = [];
  for (let i = 0; i < count; i++) {
    const size = bytes.readUInt32LE(at + 20);
    const len = bytes.readUInt16LE(at + 28);
    const extra = bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32);
    const local = bytes.readUInt32LE(at + 42);
    const name = bytes.subarray(at + 46, at + 46 + len).toString("utf8");
    const start =
      local +
      30 +
      bytes.readUInt16LE(local + 26) +
      bytes.readUInt16LE(local + 28);
    out.push({ name, data: bytes.subarray(start, start + size) });
    at += 46 + len + extra;
  }
  return out;
}

async function download(page: Page, action: () => Promise<void>) {
  const [dl] = await Promise.all([page.waitForEvent("download"), action()]);
  return {
    name: dl.suggestedFilename(),
    bytes: readFileSync((await dl.path()) as string),
  };
}

test("Édition : sélection, presse-papier, suppression, historique, clone (scène persistée) @F:menu.undoz @F:menu.redoy @F:menu.cutx @F:menu.copyc @F:menu.pastev @F:menu.delete-selected-2 @F:menu.delete-all-2 @F:menu.clone-selected-2 @F:menu.select-alla @F:menu.deselect-allesc", async ({
  page,
}) => {
  const projectId = await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await ready(page);
  const rows = page.getByTestId("object-row");
  await expect(rows).toHaveCount(2);
  await page.waitForTimeout(800);

  await edit(page, /^(Select all|Tout sélectionner)/);
  await expect(selected(page)).toHaveCount(2);
  await edit(page, /^(Deselect all|Tout désélectionner)/);
  await expect(selected(page)).toHaveCount(0);

  await page.getByTestId("object-name").first().click();
  await expect(selected(page)).toHaveCount(1);
  await edit(page, /^(Copy|Copier)/);
  await edit(page, /^(Paste|Coller)/);
  await expect(rows).toHaveCount(3);
  await expect(selected(page)).toHaveCount(1);
  await edit(page, /^(Cut|Couper)/);
  await expect(rows).toHaveCount(2);
  await edit(page, /^(Paste|Coller)/);
  await expect(rows).toHaveCount(3);
  await page.waitForTimeout(800);

  await edit(page, /^(Delete selected|Supprimer la sélection)/);
  await expect(rows).toHaveCount(2);
  await page.waitForTimeout(800);
  await edit(page, /^(Undo|Annuler)/);
  await expect(rows).toHaveCount(3);
  await edit(page, /^(Redo|Rétablir)/);
  await expect(rows).toHaveCount(2);

  await page.getByTestId("object-name").first().click();
  await edit(page, /^(Clone selected|Cloner la sélection)/);
  await expect(rows).toHaveCount(3);
  await saveProject(page, projectId);
  await page.reload();
  await expect(rows).toHaveCount(3, { timeout: 60_000 });
  await ready(page);

  await edit(page, /^(Delete all|Tout supprimer)/);
  await expect(rows).toHaveCount(0);
  await saveProject(page, projectId);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.waitForTimeout(1500);
  await expect(rows).toHaveCount(0);
});

test("Édition › Dupliquer le plateau courant : copie persistée @F:menu.duplicate-current-plate-2", async ({
  page,
}) => {
  const projectId = await bootstrap(page);
  await importModel(page, "cube20");
  await ready(page);
  await edit(page, /^(Duplicate Current Plate|Dupliquer le plateau courant)/);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await expect(page.getByTestId("plate-tab")).toHaveCount(2);
  await saveProject(page, projectId);
  const project = await (
    await page.request.get("/api/projects/" + projectId)
  ).json();
  const plates = project.scene.plates.plates as { objectIds: string[] }[];
  expect(plates).toHaveLength(2);
  expect(plates.every((p) => p.objectIds.length === 1)).toBe(true);
  expect(project.scene.objects).toHaveLength(2);
});

test("Préférences (menu Fichier et Ctrl+P) : réglages persistés après rechargement @F:menu.preferences @F:menu.preferences-2", async ({
  page,
}) => {
  await bootstrap(page);
  await menu(page, FILE, /^(Preferences|Préférences)/);
  const dialog = page.getByTestId("preferences-dialog");
  await expect(dialog).toBeVisible();
  await dialog.getByTestId("pref-units").selectOption("in");
  await dialog.getByRole("button", { name: "Fermer" }).click();
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();

  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+p");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByTestId("pref-units")).toHaveValue("in");
  const before = await dialog.getByTestId("pref-confirm-close").isChecked();
  await dialog.getByTestId("pref-confirm-close").setChecked(!before);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  const stored = JSON.parse(
    (await page.evaluate(() =>
      localStorage.getItem("web-slicer.preferences"),
    )) ?? "{}",
  );
  expect(stored.units).toBe("in");
  expect(stored.confirmClose).toBe(!before);
});

test("Preset Bundle : lot actif listé puis exporté en JSON Orca @F:menu.preset-bundle @F:menu.preset-bundle-2", async ({
  page,
}) => {
  await bootstrap(page);
  await menu(page, FILE, /^(Preset Bundle|Lot de presets|Lot de préréglages)$/);
  const dialog = page.getByTestId("preset-bundle-dialog");
  await expect(dialog).toBeVisible();
  for (const kind of ["machine", "process", "filament"])
    await expect(dialog.getByTestId("bundle-" + kind)).toHaveCount(1);
  const printerName = (await dialog
    .getByTestId("bundle-machine")
    .locator("span")
    .last()
    .textContent())!.trim();
  const zip = await download(page, () =>
    dialog.getByTestId("export-preset-bundle").click(),
  );
  expect(zip.name).toMatch(/\.zip$/);
  const entries = zipEntries(zip.bytes);
  expect(entries.length).toBe(3);
  const docs = entries.map(
    (e) =>
      JSON.parse(e.data.toString("utf8")) as { type: string; name: string },
  );
  expect(docs.map((d) => d.type).sort()).toEqual([
    "filament",
    "machine",
    "process",
  ]);
  expect(docs.find((d) => d.type === "machine")!.name).toBe(printerName);
  await expect(dialog).toBeHidden();
});

/** Ouvre un item du menu Calibration ; renvoie l'id du projet source. */
async function calib(page: Page, label: string) {
  await menu(
    page,
    CALIB,
    new RegExp("^" + label.replace(/[()/]/g, (c) => "\\" + c) + "$"),
  );
}

async function newProject(page: Page, sourceId: string) {
  await expect(page).not.toHaveURL(new RegExp(sourceId), { timeout: 180_000 });
  const id = /projects\/([^/?#]+)/.exec(page.url())![1];
  const project = await (await page.request.get("/api/projects/" + id)).json();
  return { id, project };
}

/** Couches du G-code (marqueur Orca « ;Z: » ou « ; Z_HEIGHT: ») : z et lignes. */
function gcodeLayers(gcode: string): { z: number; lines: string[] }[] {
  const layers: { z: number; lines: string[] }[] = [];
  for (const line of gcode.split("\n")) {
    const m = /^;\s*Z(?:_HEIGHT)?:\s*([\d.]+)/.exec(line);
    if (m) layers.push({ z: Number(m[1]), lines: [] });
    else layers.at(-1)?.lines.push(line);
  }
  return layers;
}

/** Projet de calibration construit par le serveur → tranché dans l'interface → G-code. */
async function sliceCalibration(page: Page, source: string) {
  const { project } = await newProject(page, source);
  expect(project.scene.objects).toHaveLength(1);
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 180_000,
  });
  await ready(page);
  await sliceInUi(page);
  return { project, gcode: await downloadGcode(page) };
}

// Calib_Temp_Tower (GCode::change_layer → interpolate_value_across_layers,
// pas de 5 °C) : une consigne M104 par couche, 9 paliers 230 → 190 de même
// nombre de couches (blocs de la tour).
test("Calibration › Temperature : tour 230→190 tranchée, paliers M104 par bloc @F:menu.temperature @F:menu.temperature-2", async ({
  page,
}) => {
  test.setTimeout(420_000);
  const source = await bootstrap(page);
  await calib(page, "Temperature");
  const dialog = page.getByTestId("calib-dialog");
  await dialog.getByTestId("calib-start").fill("230");
  await dialog.getByTestId("calib-end").fill("190");
  await dialog.getByTestId("calib-confirm").click();
  const { project, gcode } = await sliceCalibration(page, source);
  expect(project.scene.calibration?.mode).toBe("temp_tower");
  expect(gcode).not.toMatch(/^M104 S\{/m);
  // Température de chaque couche : dernière consigne M104 vue jusque-là.
  const perLayer: number[] = [];
  let current = NaN;
  for (const layer of gcodeLayers(gcode)) {
    for (const l of layer.lines) {
      const m = /^M104 S(\d+)/.exec(l);
      if (m) current = Number(m[1]);
    }
    perLayer.push(current);
  }
  expect(perLayer.length).toBeGreaterThan(20);
  for (let i = 1; i < perLayer.length; i++)
    expect(perLayer[i], "couche " + i).toBeLessThanOrEqual(perLayer[i - 1]);
  const blocks = [...new Set(perLayer)];
  expect(blocks).toEqual([230, 225, 220, 215, 210, 205, 200, 195, 190]);
  // Blocs de même hauteur (± 2 couches, arrondis de l'interpolation).
  const counts = blocks.map((t) => perLayer.filter((v) => v === t).length);
  const mean = perLayer.length / blocks.length;
  for (const [i, c] of counts.entries())
    expect(Math.abs(c - mean), `bloc ${blocks[i]} °C : ${c} couches (${counts})`).toBeLessThanOrEqual(2);
});

// Calib_PA_Tower (GCode::change_layer) : SET_PRESSURE_ADVANCE (Klipper) ou
// M900 K à chaque couche, PA = start + int(z) × step (défauts 0 → 0.1, 0.002/mm).
test("Calibration › Pressure advance : tour tranchée, PA = start + int(z) × pas à chaque couche @F:menu.pressure-advance @F:menu.pressure-advance-2", async ({
  page,
}) => {
  test.setTimeout(420_000);
  const source = await bootstrap(page);
  await calib(page, "Pressure advance");
  await page.getByTestId("calib-confirm").click();
  const { project, gcode } = await sliceCalibration(page, source);
  const params = project.scene.calibration as {
    mode: string;
    start: number;
    end: number;
    step: number;
  };
  expect(params.mode).toBe("pa_tower");
  expect([params.start, params.end, params.step]).toEqual([0, 0.1, 0.002]);
  const PA = /^(?:SET_PRESSURE_ADVANCE ADVANCE=|M900 K)([\d.]+)/;
  const layers = gcodeLayers(gcode).filter((l) => l.z > 0);
  expect(layers.length).toBeGreaterThan(100);
  const seen: number[] = [];
  for (const layer of layers) {
    const values = layer.lines
      .map((l) => PA.exec(l))
      .filter((m) => m !== null)
      .map((m) => Number(m[1]));
    expect(values.length, `couche z=${layer.z} : un réglage PA`).toBeGreaterThanOrEqual(1);
    const pa = values[0];
    // static_cast<int>(print_z) : tolérance d'un pas pour z flottant ≈ entier.
    const expected = params.start + Math.floor(layer.z + 1e-6) * params.step;
    expect(Math.abs(pa - expected), `z=${layer.z} PA=${pa}`).toBeLessThanOrEqual(params.step + 1e-6);
    seen.push(pa);
  }
  const paliers = [...new Set(seen.map((v) => Math.round(v * 1e4)))];
  expect(paliers.length).toBeGreaterThanOrEqual(40);
  expect(Math.min(...seen)).toBeCloseTo(params.start, 4);
  expect(Math.max(...seen)).toBeGreaterThan(params.end - 3 * params.step);
});

const ENGINE: {
  label: string;
  mode: string;
  values: [string, string, string?];
  tags: string;
}[] = [
  {
    label: "Max flowrate",
    mode: "vol_speed_tower",
    values: ["6", "18", "1"],
    tags: "@F:menu.max-flowrate @F:menu.max-flowrate-2",
  },
  {
    label: "Retraction test",
    mode: "retraction_tower",
    values: ["0.2", "1.4", "0.2"],
    tags: "@F:menu.retraction @F:menu.retraction-2",
  },
  {
    label: "Cornering",
    mode: "cornering",
    values: ["2", "12"],
    tags: "@F:menu.cornering @F:menu.cornering-2",
  },
  {
    label: "Input Shaping Frequency",
    mode: "input_shaping_freq",
    values: ["20", "90"],
    tags: "@F:menu.input-shaping-frequency @F:menu.input-shaping-frequency-2",
  },
  {
    label: "Input Shaping Damping/zeta factor",
    mode: "input_shaping_damp",
    values: ["0.05", "0.35"],
    tags: "@F:menu.input-shaping-damping-zeta-factor @F:menu.input-shaping-damping-zeta-factor-2",
  },
  {
    label: "VFA",
    mode: "vfa_tower",
    values: ["40", "120", "20"],
    tags: "@F:menu.vfa @F:menu.vfa-2",
  },
];

for (const c of ENGINE) {
  test(
    "Calibration › " +
      c.label +
      " : projet calibré persisté (mode moteur " +
      c.mode +
      ") " +
      c.tags,
    async ({ page }) => {
      test.setTimeout(300_000);
      const source = await bootstrap(page);
      await calib(page, c.label);
      const dialog = page.getByTestId("engine-calib-dialog");
      await expect(dialog).toBeVisible();
      await dialog.getByTestId("calib-start").fill(c.values[0]);
      await dialog.getByTestId("calib-end").fill(c.values[1]);
      if (c.values[2]) await dialog.getByTestId("calib-step").fill(c.values[2]);
      await dialog.getByTestId("engine-calib-confirm").click();
      const { project } = await newProject(page, source);
      const cal = project.scene.calibration as Record<string, number | string>;
      expect(cal.mode).toBe(c.mode);
      if (c.mode === "input_shaping_freq") {
        expect(Number(cal.freq_start_x)).toBeCloseTo(Number(c.values[0]));
        expect(Number(cal.freq_end_x)).toBeCloseTo(Number(c.values[1]));
      } else if (c.mode === "vol_speed_tower") {
        // Orca convertit les mm³/s en mm/s (section de ligne) : rapport conservé.
        expect(Number(cal.start)).toBeGreaterThan(Number(c.values[0]));
        expect(Number(cal.end) / Number(cal.start)).toBeCloseTo(
          Number(c.values[1]) / Number(c.values[0]),
          3,
        );
      } else {
        expect(Number(cal.start)).toBeCloseTo(Number(c.values[0]));
        expect(Number(cal.end)).toBeCloseTo(Number(c.values[1]));
      }
      expect(project.scene.objects).toHaveLength(1);
      await expect(page.getByTestId("object-row")).toHaveCount(1, {
        timeout: 120_000,
      });
    },
  );
}

test("Calibration › Flow ratio : YOLO (recommandé), 11 objets à ratio absolu, motif Archimedean @F:menu.flow-ratio @F:menu.flow-ratio-2", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const source = await bootstrap(page);
  await calib(page, "Flow ratio");
  const dialog = page.getByTestId("flow-calib-dialog");
  await expect(dialog.getByTestId("flow-type-2")).toBeChecked();
  await expect(dialog.getByTestId("flow-pattern")).toHaveValue(
    "archimedeanchords",
  );
  await dialog.getByTestId("flow-calib-confirm").click();
  const { project } = await newProject(page, source);
  const objects = project.scene.objects as {
    settings: Record<string, unknown>;
  }[];
  expect(objects).toHaveLength(11);
  const filamentId = (project.active_presets.filaments as string[])[0];
  const resolved = await (
    await page.request.get("/api/presets/" + filamentId + "/resolved")
  ).json();
  const raw = resolved.values.filament_flow_ratio;
  const r = Number(Array.isArray(raw) ? raw[0] : raw) || 1;
  const ratios = objects
    .map((o) => Number(o.settings.print_flow_ratio))
    .sort((a, b) => a - b);
  const expected = [
    -0.05, -0.04, -0.03, -0.02, -0.01, 0, 0.01, 0.02, 0.03, 0.04, 0.05,
  ].map((m) => (r + m) / r);
  ratios.forEach((v, i) => expect(v).toBeCloseTo(expected[i], 4));
  expect(
    objects.every(
      (o) => o.settings.top_surface_pattern === "archimedeanchords",
    ),
  ).toBe(true);
  await expect(page.getByTestId("object-row")).toHaveCount(11, {
    timeout: 120_000,
  });
});

test("Calibration › Calibration Guide : ouvre le guide Orca dans un nouvel onglet @F:menu.calibration-guide @F:menu.calibration-guide-2", async ({
  page,
  context,
}) => {
  await context.route("https://www.orcaslicer.com/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<title>Calibration guide</title>",
    }),
  );
  await bootstrap(page);
  const [tab] = await Promise.all([
    context.waitForEvent("page"),
    calib(page, "Calibration Guide"),
  ]);
  await tab.waitForLoadState();
  expect(tab.url()).toBe("https://www.orcaslicer.com/wiki/calibration_guide");
  await expect(tab).toHaveTitle("Calibration guide");
});

test("G-code : ouvrir, recharger (menu et F5), exporter les trajets OBJ @F:menu.open-g-code @F:menu.reload-from-disk @F:menu.reload-from-diskf5 @F:menu.export-toolpaths-as-obj-2", async ({
  page,
}) => {
  await bootstrap(page);
  const [chooser] = await Promise.all([
    page.waitForEvent("filechooser"),
    menu(page, FILE, /open g-code|ouvrir.*g-code/i),
  ]);
  await chooser.setFiles(GCODE);
  const preview = page.getByTestId("preview-view");
  await expect(preview).toHaveAttribute("data-layer-count", /^[1-9]\d*$/, {
    timeout: 30_000,
  });
  const layers = Number(await preview.getAttribute("data-layer-count"));
  const slider = page.getByTestId("layer-slider");
  await expect(slider).toHaveAttribute(
    "data-current-layer",
    String(layers - 1),
  );

  // Reload from Disk… : le fichier est relu et ré-analysé (aperçu réinitialisé).
  await slider.fill("0");
  await expect(slider).toHaveAttribute("data-current-layer", "0");
  const reparse = page.waitForRequest(
    (r) => r.method() === "POST" && /meta=1/.test(r.url()),
  );
  await menu(page, FILE, /reload from disk|recharger depuis le disque/i);
  await reparse;
  await expect(slider).toHaveAttribute(
    "data-current-layer",
    String(layers - 1),
  );

  // F5 : même commande, sans rechargement de la page.
  await slider.fill("0");
  await expect(slider).toHaveAttribute("data-current-layer", "0");
  await page.evaluate(
    () => ((window as unknown as { __menuB: number }).__menuB = 1),
  );
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  const reparseF5 = page.waitForRequest(
    (r) => r.method() === "POST" && /meta=1/.test(r.url()),
  );
  await page.keyboard.press("F5");
  await reparseF5;
  await expect(slider).toHaveAttribute(
    "data-current-layer",
    String(layers - 1),
  );
  expect(
    await page.evaluate(
      () => (window as unknown as { __menuB?: number }).__menuB,
    ),
  ).toBe(1);

  const obj = await download(page, () =>
    menu(page, FILE, /toolpaths/i, /^(export|exporter)\b/i),
  );
  expect(obj.name).toMatch(/\.obj$/);
  const text = obj.bytes.toString("utf8");
  const vertices = (text.match(/^v /gm) ?? []).length;
  expect(vertices).toBeGreaterThan(100);
  expect((text.match(/^l /gm) ?? []).length).toBe(vertices / 2);
});
