// Phase 44b — interface : type de plaque par défaut = celui de l'imprimante,
// « Ouvrir un G-code » sur un fichier tranché .gcode.3mf (sélecteur de
// plateau), projet .gcode.3mf refusé → proposition d'aperçu G-code, données
// partagées par les instances (oreilles de bordure), formes génériques qui
// annulent l'échelle de l'objet, Aperçu utilisable à 760 × 490.
//
// Oracle Orca 2.4.1 : Preset::get_default_bed_type (Preset.cpp:966) appliqué
// par Sidebar::update_presets (Plater.cpp:2513-2540) ; Plater::is_gcode_3mf
// (aperçu d'un .gcode.3mf, Metadata/plate_<n>.gcode) ; ModelObject::
// brim_points partagés par les ModelInstance ; ObjectList::
// load_generic_subobject → Transformation::volume_to_bed_transformation.
//
// ⚠ Lot 44 serveur requis (backend redémarré) pour : « tranchage avec le type
// de plaque », « .gcode.3mf ouvert », « projet .gcode.3mf ». Les autres tests
// ne dépendent que de l'interface.
import { expect, test, type Page } from "@playwright/test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { bootstrap, downloadGcode, importModel, sliceInUi } from "../lib/app";

type Plate = { id: string; plateType: string };
type SceneObject = {
  transform: { position: number[]; scale: number[] };
  brimEars?: { points: number[][] };
  instanceGroup?: string;
  volumes?: { name: string; shape: string; size: number[]; offset: number[] }[];
};
type SceneDoc = { objects: SceneObject[]; plates: { plates: Plate[] } };

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  return id;
}
async function saveProject(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse((r) => r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url())),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
}
async function scene(page: Page, id: string): Promise<SceneDoc> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return ((await res.json()) as { scene: SceneDoc }).scene;
}

// — Archive ZIP minimale (méthode « stocké ») : .gcode.3mf sans géométrie,
//   comme store_bbs_3mf(SkipModel) d'Orca (Metadata/plate_<n>.gcode). —
const CRC = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function zip(entries: Record<string, string>): Buffer {
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;
  for (const [name, text] of Object.entries(entries)) {
    const data = Buffer.from(text, "utf8");
    const fname = Buffer.from(name, "utf8");
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(fname.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(fname.length, 28);
    central.writeUInt32LE(offset, 42);
    locals.push(local, fname, data);
    centrals.push(central, fname);
    offset += 30 + fname.length + data.length;
  }
  const cd = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(Object.keys(entries).length, 8);
  end.writeUInt16LE(Object.keys(entries).length, 10);
  end.writeUInt32LE(cd.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, end]);
}
/** G-code de `layers` couches (même forme que le G-code externe de phase31). */
function gcode(layers: number, x0: number): string {
  const lines = ["G28", "G90", "M83"];
  for (let l = 0; l < layers; l++) {
    const z = (0.2 * (l + 1)).toFixed(2);
    lines.push(";LAYER_CHANGE", `;Z:${z}`, `G1 Z${z} F600`, ";TYPE:Outer wall", ";WIDTH:0.45", ";HEIGHT:0.2");
    lines.push(`G1 X${x0} Y100 F3000`, `G1 X${x0 + 20} Y100 E1 F1800`, `G1 X${x0 + 20} Y120 E1`, `G1 X${x0} Y120 E1`);
  }
  lines.push(`; total layers count = ${layers}`, "; estimated printing time (normal mode) = 1h 2m 3s");
  return lines.join("\n") + "\n";
}
function sliced3mf(): string {
  const dir = mkdtempSync(join(tmpdir(), "p44b-"));
  const file = join(dir, "deux-plateaux.gcode.3mf");
  writeFileSync(
    file,
    zip({
      "[Content_Types].xml": '<?xml version="1.0" encoding="UTF-8"?><Types/>',
      "3D/3dmodel.model": '<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter"><resources/><build/></model>',
      "Metadata/plate_1.gcode": gcode(4, 100),
      "Metadata/plate_2.gcode": gcode(6, 150),
    }),
  );
  return file;
}

test.describe("phase 44b (FR)", () => {
  test.use({ locale: "fr-FR" });

  // ─── Type de plaque par défaut ────────────────────────────────────────────
  test("type de plaque : un nouveau projet / plateau prend celui de l'imprimante (Voron → plaque haute température lisse), affiché dans les réglages du plateau, persisté @F:editor.plate-type @F:editor.plate-settings", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    const type = page.getByRole("combobox", { name: "Type de plaque", exact: true });
    // Orca : get_default_bed_type d'une imprimante sans default_bed_type (hors X1) = btPEI.
    await expect(type).toHaveValue("Smooth High Temp Plate", { timeout: 30_000 });
    // Réglages du plateau : l'option « suivre » affiche ce type (Same as Global Plate Type).
    await page.getByTestId("plate-settings-button").click();
    const dialog = page.getByTestId("settings-dialog");
    await expect(dialog.locator('[data-param="curr_bed_type"] select option').first()).toHaveText(
      "Profil (Plaque haute température lisse)",
    );
    await dialog.getByRole("button", { name: "Fermer" }).click();
    // Nouveau plateau : même type.
    await page.getByRole("button", { name: "Ajouter un plateau" }).click();
    await expect(type).toHaveValue("Smooth High Temp Plate");
    // Un type choisi explicitement est conservé.
    await type.selectOption("Textured PEI Plate");
    await saveProject(page);
    const doc = await scene(page, id);
    expect(doc.plates.plates.map((p) => p.plateType)).toEqual(["Smooth High Temp Plate", "Textured PEI Plate"]);
    // Rechargement : types relus tels quels (Orca ne touche pas au type d'un projet chargé).
    await page.reload();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    // Plateau actif enregistré = plateau 2 (type choisi) ; le plateau 1 garde celui de l'imprimante.
    const reloaded = page.getByRole("combobox", { name: "Type de plaque", exact: true });
    await expect(reloaded).toHaveValue("Textured PEI Plate", { timeout: 60_000 });
    await page
      .getByRole("complementary", { name: "Configuration" })
      .getByRole("button", { name: /^Plateau 1\b/ })
      .click();
    await expect(reloaded).toHaveValue("Smooth High Temp Plate");
    await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
  });

  test("type de plaque : le tranchage utilise la plaque de l'imprimante (curr_bed_type = High Temp Plate, M190 = hot_plate_temp_initial_layer) — lot 44 requis @F:editor.plate-type", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await expect(page.getByRole("combobox", { name: "Type de plaque", exact: true })).toHaveValue(
      "Smooth High Temp Plate",
      {
        timeout: 30_000,
      },
    );
    await sliceInUi(page);
    const text = await downloadGcode(page);
    expect(text).toMatch(/^; curr_bed_type = High Temp Plate$/m);
    const hot = /^; hot_plate_temp_initial_layer = (\d+)/m.exec(text)?.[1];
    expect(hot).toBeTruthy();
    const bed = /^M190 S(\d+)/m.exec(text) ?? /^M140 S(\d+)/m.exec(text);
    expect(bed?.[1]).toBe(hot!.split(",")[0]);
  });

  // ─── .gcode.3mf ────────────────────────────────────────────────────────────
  test("Ouvrir un G-code : .gcode.3mf à deux plateaux → aperçu du plateau 1, sélecteur de plateau → plateau 2 — lot 44 requis @F:editor.menu-open-gcode", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await bootstrap(page);
    await expect(page.getByTestId("gcode-open-input")).toHaveAttribute("accept", /\.3mf/);
    await page.getByTestId("gcode-open-input").setInputFiles(sliced3mf());
    await expect(page.getByTestId("editor-message")).toContainText("deux-plateaux.gcode.3mf", { timeout: 30_000 });
    await expect(page.getByTestId("editor-message")).toContainText("plateau 1");
    await expect(page.getByTestId("preview-canvas")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("layer-slider")).toHaveAttribute("max", "3");
    const select = page.getByTestId("gcode-plate-select");
    await expect(select.locator("option")).toHaveText(["Plateau 1", "Plateau 2"]);
    await select.selectOption("2");
    await expect(page.getByTestId("editor-message")).toContainText("plateau 2", { timeout: 30_000 });
    await expect(page.getByTestId("layer-slider")).toHaveAttribute("max", "5");
    await expect(select).toHaveValue("2");
  });

  test("Ouvrir un projet .gcode.3mf : refus explicite du serveur (422 gcode_3mf) → proposition d'aperçu G-code — lot 44 requis @F:editor.menu-open-project @F:editor.menu-open-gcode", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await bootstrap(page);
    await page.getByTestId("project-open-input").setInputFiles(sliced3mf());
    const ask = page.getByTestId("open3mf-dialog");
    const offer = page.getByTestId("gcode3mf-offer");
    await expect(ask.or(offer)).toBeVisible({ timeout: 30_000 });
    if (await ask.isVisible()) await page.getByTestId("open3mf-ok").click();
    await expect(offer).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("gcode3mf-offer-text")).toContainText("deux-plateaux.gcode.3mf");
    await expect(page.getByTestId("gcode3mf-offer-text")).toContainText("2 plateau(x)");
    await page.getByTestId("gcode3mf-offer-open").click();
    await expect(offer).toHaveCount(0);
    await expect(page.getByTestId("preview-canvas")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("gcode-plate-select")).toHaveValue("1");
    // Aucun projet créé : on reste sur le projet courant.
    expect(page.url()).toMatch(/\/projects\/[^/]+$/);
  });

  // ─── Instances : oreilles de bordure partagées ─────────────────────────────
  test("instances : une oreille de bordure posée sur une instance est reportée sur l'autre, dans son repère (ModelObject::brim_points) @F:gizmo.brimears @F:ctx.add-instance", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("object-row").first().click({ button: "right" });
    await page.locator('[role=menu] [data-action="object.addInstance"]').click();
    await expect(page.getByTestId("object-row")).toHaveCount(2);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-brim-ears").click();
    const canvas = page.getByTestId("scene-canvas");
    await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
    const [x, y] = (await canvas.getAttribute("data-object-anchor"))!.split(",").map(Number);
    const box = (await canvas.boundingBox())!;
    await page.mouse.click(box.x + x, box.y + y);
    await expect(page.getByRole("button", { name: "Retirer l'oreille 1" })).toBeVisible();
    // L'autre instance porte la même oreille.
    await page.getByTestId("object-name").nth(1).click();
    await expect(page.getByRole("button", { name: "Retirer l'oreille 1" })).toBeVisible();
    await saveProject(page);
    const doc = await scene(page, id);
    const [a, b] = doc.objects;
    expect(a.instanceGroup).toBeTruthy();
    expect(b.instanceGroup).toBe(a.instanceGroup);
    expect(a.brimEars?.points).toHaveLength(1);
    expect(b.brimEars?.points).toHaveLength(1);
    // Même point dans le repère de chaque instance : décalé comme les instances.
    const dx = b.transform.position[0] - a.transform.position[0];
    const dy = b.transform.position[1] - a.transform.position[1];
    expect(b.brimEars!.points[0][0] - a.brimEars!.points[0][0]).toBeCloseTo(dx, 3);
    expect(b.brimEars!.points[0][1] - a.brimEars!.points[0][1]).toBeCloseTo(dy, 3);
  });

  // ─── Formes génériques sur un objet mis à l'échelle ───────────────────────
  test("forme générique (Cube) sur un objet ×2 : échelle de l'objet annulée comme Orca (côté 35 mm à l'écran = 17,5 mm dans le repère de l'objet) @F:editor.volume-modifier", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    await page.getByLabel("Échelle X", { exact: true }).fill("200");
    await page.getByLabel("Échelle X", { exact: true }).press("Tab");
    await page.getByTestId("object-row").first().click({ button: "right" });
    await page.getByTestId("menu-sub-object.addModifier").click();
    await page
      .getByTestId("menu-shapes-object.addModifier")
      .getByRole("menuitem", { name: "Cube", exact: true })
      .click();
    await expect(page.getByText(/Générique-Cube/).first()).toBeVisible({ timeout: 30_000 });
    await saveProject(page);
    const doc = await scene(page, id);
    expect(doc.objects[0].transform.scale).toEqual([2, 2, 2]);
    const cube = (doc.objects[0].volumes ?? []).find((v) => v.name === "Générique-Cube");
    expect(cube?.shape).toBe("mesh");
    // Voron 350 : côté Orca = 0,1 × 350 = 35 mm à l'écran → 17,5 mm sous l'échelle ×2.
    for (const s of cube!.size) expect(s).toBeCloseTo(17.5, 1);
    // Coin (max.x, min.y, min.z) + ½ côté à l'écran, ramené dans le repère de l'objet (÷ 2).
    expect(cube!.offset[0]).toBeCloseTo(18.75, 1);
    expect(cube!.offset[1]).toBeCloseTo(-1.25, 1);
    expect(cube!.offset[2]).toBeCloseTo(-1.25, 1);
  });

  // ─── Aperçu à 760 × 490 ────────────────────────────────────────────────────
  test("Aperçu à 760 × 490 : pas de défilement de la page, curseur de couches utilisable, légende sous la barre sans recouvrir le navigateur 3D @F:ui.preview-layout", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    await ready(page, ["cube20"]);
    await sliceInUi(page);
    await page.setViewportSize({ width: 760, height: 490 });
    await page.waitForTimeout(500);
    const dims = await page.evaluate(() => ({
      sw: document.documentElement.scrollWidth,
      sh: document.documentElement.scrollHeight,
      vw: innerWidth,
      vh: innerHeight,
    }));
    expect(dims.sw).toBeLessThanOrEqual(dims.vw);
    expect(dims.sh).toBeLessThanOrEqual(dims.vh);
    const slider = (await page.getByTestId("layer-slider").boundingBox())!;
    expect(slider.height).toBeGreaterThanOrEqual(100);
    const legend = (await page.getByTestId("preview-legend-panel").boundingBox())!;
    const cube = await page.getByTestId("preview-view-cube").boundingBox();
    if (cube) expect(legend.y + legend.height).toBeLessThanOrEqual(cube.y + 1);
    // Barre de coloration : tous ses boutons cliquables (aucun sous la légende).
    await page.getByTestId("preview-fit-bed").click();
    await expect(page.getByTestId("preview-fit-bed")).toHaveAttribute("aria-pressed", "true");
    await page.getByTestId("preview-render-lines").click();
    await expect(page.getByTestId("preview-render-lines")).toHaveAttribute("aria-pressed", "true");
    // « Aller à… » atteignable (panneau des couches défilant).
    await page.getByTestId("preview-goto-layer-open").scrollIntoViewIfNeeded();
    await page.getByTestId("preview-goto-layer-open").click();
    await expect(page.getByTestId("preview-goto-layer")).toBeVisible();
  });
});
