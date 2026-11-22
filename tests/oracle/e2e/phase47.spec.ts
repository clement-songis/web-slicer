// Phase 47 — corrections de l'audit 45 (navigateur), partie interface :
// volumes de purge = matrice du tranchage, cellules modifiées seules (MM-a),
// séquences de filaments du plateau « Personnaliser » (MM-b), Disposer / A en
// objet par objet avec le rayon de dégagement (SEQ-a), outil Oreilles de
// bordure complet et oreilles envoyées avec la scène (EARS-a/b), actions de
// glisser par bouton et sauvegarde automatique (PREF-a/b).
//
// Oracle Orca 2.4.1 : WipingDialog (matrice du projet, « Re-calculate »),
// PlateSettingsDialog.cpp (DragCanvas, OtherLayersSeqPanel, LayerSeqInfo),
// ArrangeJob::prepare_all / init_arrange_params (m_uncompatible_plates,
// is_seq_print), GLGizmoBrimEars (Head diameter, Max angle, Detection radius,
// Auto-generate, Section view, avertissements), Preferences.cpp
// (left/middle/right_mouse_drag_action, backup_switch / backup_interval).
//
// CONTRAT: flush-matrix[data-source], flush-source-auto, plate-first-layer-order,
// plate-other-layers-sequence, plate-layer-range*, brim-ears-tool, brim-ears-head,
// brim-ears-angle, brim-ears-radius, brim-ears-section, brim-ears-auto,
// brim-ears-warning, brim-ears-set-painted, brim-ear-row, scene-canvas[data-guide-ears],
// pref-auto-backup, pref-backup-interval, backup-restore-dialog, backup-restore-yes.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

type Plate = { settings?: Record<string, unknown> };
type Obj = {
  name?: string;
  transform: { position: number[] };
  settings?: Record<string, unknown>;
  brimEars?: { points: number[][]; ears?: number[][] };
};
type SceneDoc = {
  objects: Obj[];
  plates?: { plates?: Plate[] };
  projectConfig?: { flush_volumes_matrix?: number[]; flush_multiplier?: number };
};

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  return id;
}
async function scene(page: Page, id: string): Promise<SceneDoc> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return ((await res.json()) as { scene: SceneDoc }).scene;
}
async function saveProject(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse((r) => r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url())),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status()).toBeLessThan(300);
}
async function swatchColours(page: Page): Promise<string[]> {
  return page.getByTestId("filament-swatch").evaluateAll((els) => els.map((e) => e.getAttribute("data-colour") ?? ""));
}

test.describe("audit 45 (FR)", () => {
  test("MM-a : le dialogue des volumes de purge montre la matrice calculée depuis les couleurs (celle du tranchage) ; Appliquer ne réécrit que les cellules modifiées @F:editor.multi-filament", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await page.getByTestId("add-filament-slot").click();
    await page.getByTestId("add-filament-slot").click();
    await expect(page.getByTestId("filament-swatch")).toHaveCount(3);
    const colours = await swatchColours(page);
    // Matrice que le moteur calcule pour ces couleurs (projet : minimum de purge…).
    const res = await page.request.post("/api/flush-volumes", {
      data: { colours, multiplier: 1, project_id: id },
    });
    expect(res.status()).toBe(200);
    const computed = ((await res.json()) as { matrix: number[] }).matrix.map((v) => Math.round(v));
    expect(computed).toHaveLength(9);

    await page.getByTestId("open-flush-volumes").click();
    const dlg = page.getByTestId("flush-dialog");
    await expect(dlg.getByTestId("flush-matrix")).toHaveAttribute("data-source", "auto");
    await expect(dlg.getByTestId("flush-source-auto")).toBeVisible();
    await expect(dlg.getByTestId("flush-compute")).toHaveText("Re-calculer");
    for (const [r, c] of [[1, 2], [1, 3], [2, 1], [2, 3], [3, 1], [3, 2]])
      await expect(dlg.getByTestId(`flush-cell-${r}-${c}`)).toHaveValue(String(computed[(r - 1) * 3 + (c - 1)]));
    // Rien de modifié : aucune matrice écrite (le tranchage continue de la calculer).
    await dlg.getByTestId("flush-apply").click();
    await expect(dlg).toHaveCount(0);
    await saveProject(page);
    expect((await scene(page, id)).projectConfig?.flush_volumes_matrix).toBeUndefined();

    // Une cellule modifiée : seules ses valeurs changent.
    await page.getByTestId("open-flush-volumes").click();
    await expect(dlg.getByTestId("flush-matrix")).toHaveAttribute("data-source", "auto");
    await dlg.getByTestId("flush-cell-1-2").fill("500");
    await dlg.getByTestId("flush-cell-1-2").press("Tab");
    await dlg.getByTestId("flush-apply").click();
    await saveProject(page);
    const expected = [...computed];
    expected[1] = 500;
    expect((await scene(page, id)).projectConfig?.flush_volumes_matrix).toEqual(expected);
    // Réouverture : matrice du projet affichée telle quelle.
    await page.getByTestId("open-flush-volumes").click();
    await expect(dlg.getByTestId("flush-matrix")).toHaveAttribute("data-source", "project");
    await expect(dlg.getByTestId("flush-cell-2-3")).toHaveValue(String(computed[5]));
  });

  test("MM-b : séquence de la 1ʳᵉ couche et plages des autres couches « Personnaliser » éditables, écrites au format d'Orca @F:editor.plate-settings", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await page.setViewportSize({ width: 800, height: 500 });
    const id = await ready(page, ["cube20"]);
    await page.getByTestId("add-filament-slot").click();
    await page.getByTestId("plate-settings-button").first().click();
    const dialog = page.getByTestId("settings-dialog");
    await dialog.getByTestId("settings-mode-develop").click();
    await dialog.locator("[data-param=first_layer_sequence_choice] select").selectOption("Customize");
    const first = dialog.getByTestId("plate-first-layer-order");
    await expect(first.locator("li")).toHaveCount(2);
    await first.getByRole("button", { name: "Imprimer le filament 1 plus tard" }).click();
    await expect(first.locator("li").first()).toHaveAttribute("data-filament", "2");
    await dialog.locator("[data-param=other_layers_sequence_choice] select").selectOption("Customize");
    const other = dialog.getByTestId("plate-other-layers-sequence");
    await expect(other.getByTestId("plate-layer-range")).toHaveCount(1);
    await other.getByTestId("plate-layer-range-add").click();
    const ranges = other.getByTestId("plate-layer-range");
    await expect(ranges).toHaveCount(2);
    // 2ᵉ plage : 11 à Fin, ordre inversé ; 1ʳᵉ : couches 2 à 10 (plages triées
    // par début / fin à chaque saisie, comme EVT_NEED_RESORT_LAYERS d'Orca).
    await ranges.nth(1).getByTestId("plate-layer-range-begin").fill("11");
    await ranges.nth(1).getByTestId("plate-layer-range-begin").press("Tab");
    await ranges.nth(1).getByTestId("plate-layer-range-end-kind").selectOption("end");
    await ranges.nth(1).getByRole("button", { name: "Imprimer le filament 2 plus tôt" }).click();
    await ranges.nth(0).getByTestId("plate-layer-range-end-kind").selectOption("custom");
    await ranges.nth(0).getByTestId("plate-layer-range-end").fill("10");
    await ranges.nth(0).getByTestId("plate-layer-range-end").press("Tab");
    await expect(ranges.nth(0).getByTestId("plate-layer-range-end")).toHaveValue("10");
    // Contenu dans le dialogue (plus de liste qui déborde).
    const box = (await dialog.boundingBox())!;
    for (const sel of await dialog.locator("select").all()) {
      const b = await sel.boundingBox();
      if (b) expect(b.x + b.width).toBeLessThanOrEqual(box.x + box.width + 1);
    }
    await dialog.getByTestId("settings-apply").click();
    await saveProject(page);
    const settings = (await scene(page, id)).plates?.plates?.[0]?.settings ?? {};
    expect(settings.first_layer_sequence_choice).toBe("Customize");
    expect(settings.first_layer_print_sequence).toBe("2,1");
    expect(settings.other_layers_sequence_choice).toBe("Customize");
    expect(settings.other_layers_print_sequence).toBe("2,10,1,2,11,2147483646,2,1");
    expect(settings.other_layers_print_sequence_nums).toBe("2");
    // Retour à « Auto » : séquences retirées (PartPlate::set_*_print_sequence({})).
    await page.getByTestId("plate-settings-button").first().click();
    await expect(dialog.getByTestId("plate-first-layer-order").locator("li").first()).toHaveAttribute("data-filament", "2");
    await dialog.locator("[data-param=first_layer_sequence_choice] select").selectOption("Auto");
    await dialog.getByTestId("settings-apply").click();
    await saveProject(page);
    const after = (await scene(page, id)).plates?.plates?.[0]?.settings ?? {};
    expect(after.first_layer_print_sequence).toBeUndefined();
    expect(after.other_layers_print_sequence).toBe("2,10,1,2,11,2147483646,2,1");
  });

  test("SEQ-a : plateau « par objet » sous un traitement « par couche » — A espace les objets au rayon de dégagement (plateau arrangé avec sa séquence) @F:toolbar.arrange @F:editor.arrange-sequential", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20", "cylinder", "bridge"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("plate-settings").getByTestId("plate-setting-print_sequence").selectOption("by object");
    await saveProject(page);
    const before = (await scene(page, id)).objects.map((o) => o.transform.position.slice(0, 2));
    const bodies: Record<string, unknown>[] = [];
    page.on("request", (r) => {
      if (/\/arrange$/.test(r.url()) && r.method() === "POST") bodies.push(r.postDataJSON());
    });
    await page.getByTestId("scene-canvas").click({ position: { x: 30, y: 30 } });
    await page.locator("body").focus();
    await page.keyboard.press("a");
    await expect.poll(() => bodies.length, { timeout: 60_000 }).toBe(2);
    expect(bodies[0].scope).toBe("all");
    expect(bodies[0].is_seq_print).toBe(false);
    expect(bodies[1].scope).toBe("plate");
    expect(Number(bodies[1].extruder_clearance_radius)).toBeGreaterThan(0);
    await expect(page.getByTestId("editor-message")).toContainText(/Arrangement/, { timeout: 60_000 });
    await saveProject(page);
    const after = (await scene(page, id)).objects.map((o) => o.transform.position.slice(0, 2));
    expect(after).not.toEqual(before);
    // Objets écartés d'au moins le rayon de dégagement (centres > 45 mm ici).
    let min = Infinity;
    for (let i = 0; i < after.length; i++)
      for (let j = i + 1; j < after.length; j++)
        min = Math.min(min, Math.hypot(after[i][0] - after[j][0], after[i][1] - after[j][1]));
    expect(min).toBeGreaterThan(45);
  });

  test("EARS-b/a : outil Oreilles complet (diamètre, angle, rayon, auto-génération, coupe, avertissement « peinte ») ; oreilles dessinées, listées et enregistrées dans le repère de l'objet @F:gizmo.brimears", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-brim-ears").click();
    const tool = page.getByTestId("brim-ears-tool");
    await expect(tool).toBeVisible();
    for (const t of ["brim-ears-head", "brim-ears-angle", "brim-ears-radius", "brim-ears-section"])
      await expect(tool.getByTestId(t)).toBeVisible();
    await expect(tool.getByTestId("brim-ears-angle")).toHaveValue("125");
    await expect(tool.getByTestId("brim-ears-radius")).toHaveValue("1");
    // Bordure du profil ≠ « peinte » : avertissement d'Orca + lien.
    await expect(tool.getByTestId("brim-ears-warning")).toContainText("peint");
    await tool.getByTestId("brim-ears-set-painted").click();
    await expect(tool.getByTestId("brim-ears-warning")).toHaveCount(0);
    // Auto-générer : les 4 coins du cube, listés et dessinés.
    await tool.getByTestId("brim-ears-head").fill("10");
    await tool.getByTestId("brim-ears-head").press("Tab");
    await tool.getByTestId("brim-ears-auto").click();
    await expect(tool.getByTestId("brim-ear-row")).toHaveCount(4);
    const canvas = page.getByTestId("scene-canvas");
    await expect(canvas).toHaveAttribute("data-guide-ears", "4");
    // Un clic sur le modèle ajoute une 5ᵉ oreille (liste à jour à chaque clic).
    await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
    const [x, y] = (await canvas.getAttribute("data-object-anchor"))!.split(",").map(Number);
    const cb = (await canvas.boundingBox())!;
    await page.mouse.click(cb.x + x, cb.y + y);
    await expect(tool.getByTestId("brim-ear-row")).toHaveCount(5);
    // Vue en coupe.
    await tool.getByTestId("brim-ears-section").fill("0.5");
    await tool.getByTestId("brim-ears-section").press("Tab");
    await expect(canvas).toHaveAttribute("data-section-normal", /.+/);
    // Retirer la sélection (dernière oreille posée).
    await tool.getByTestId("brim-ears-remove-selected").click();
    await expect(tool.getByTestId("brim-ear-row")).toHaveCount(4);
    await saveProject(page);
    const doc = await scene(page, id);
    const obj = doc.objects[0];
    expect(obj.settings?.brim_type).toBe("painted");
    // EARS-a : envoyées avec la scène — repère objet + rayon (Ø 10 → 5) et
    // positions plateau aux coins du cube (±10 mm du centre).
    expect(obj.brimEars?.ears).toHaveLength(4);
    for (const e of obj.brimEars!.ears!) {
      expect(Math.abs(Math.abs(e[0]) - 10)).toBeLessThan(0.3);
      expect(Math.abs(Math.abs(e[1]) - 10)).toBeLessThan(0.3);
      expect(e[3]).toBeCloseTo(5, 3);
    }
    const [cx, cy] = obj.transform.position;
    // Format lu par le serveur (scene_brim_ears) : [x, y, diamètre] plateau.
    for (const p of obj.brimEars!.points) {
      expect(Math.abs(Math.abs(p[0] - cx) - 10)).toBeLessThan(0.3);
      expect(Math.abs(Math.abs(p[1] - cy) - 10)).toBeLessThan(0.3);
      expect(p[2]).toBeCloseTo(10, 3);
    }
  });

  test("PREF-a/b : actions de glisser par bouton au libellé d'Orca ; sauvegarde automatique — brouillon proposé et restauré après fermeture sans enregistrer @F:menu.preferences", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    const id = await ready(page, ["cube20"]);
    await saveProject(page);
    await page.getByRole("button", { name: /^(file|fichier)$/i }).first().click();
    await page.getByRole("menuitem", { name: /pr[ée]f[ée]rences/i }).first().click();
    const dialog = page.getByTestId("preferences-dialog");
    await dialog.getByTestId("pref-tab-camera").click();
    await expect(dialog.getByText("Glisser avec le bouton gauche de la souris")).toBeVisible();
    await expect(dialog.getByText("Glisser avec le bouton droit de la souris")).toBeVisible();
    await expect(dialog.getByTestId("pref-leftDrag").locator("option")).toHaveText(["Aucun", "Panoramique", "Pivoter"]);
    await dialog.getByTestId("pref-tab-general").click();
    await expect(dialog.getByTestId("pref-auto-backup")).toBeChecked();
    await expect(dialog.getByTestId("pref-backup-interval")).toHaveValue("10");
    await dialog.getByTestId("pref-backup-interval").fill("1");
    await dialog.getByTestId("pref-backup-interval").press("Tab");
    await page.keyboard.press("Escape");
    // Modification non enregistrée : une instance de plus (clavier « + »).
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("scene-canvas").click({ position: { x: 30, y: 30 } });
    await page.getByTestId("object-name").first().click();
    await page.locator("body").focus();
    await page.keyboard.press("+");
    await expect(page.getByTestId("object-row")).toHaveCount(2);
    await expect
      .poll(() => page.evaluate((k) => localStorage.getItem(k) !== null, `web-slicer.backup.${id}`), { timeout: 15_000 })
      .toBe(true);
    expect((await scene(page, id)).objects).toHaveLength(1);
    // « Fermeture » de l'onglet : rechargement sans enregistrer.
    page.on("dialog", (d) => void d.accept());
    await page.reload();
    const restore = page.getByTestId("backup-restore-dialog");
    await expect(restore).toBeVisible({ timeout: 60_000 });
    await expect(restore).toContainText("restaurer");
    await restore.getByTestId("backup-restore-yes").click();
    await expect.poll(async () => (await scene(page, id)).objects.length, { timeout: 30_000 }).toBe(2);
    await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 60_000 });
    await expect(page.getByTestId("backup-restore-dialog")).toHaveCount(0);
  });
});
