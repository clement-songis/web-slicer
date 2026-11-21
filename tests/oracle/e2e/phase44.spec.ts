// Phase 44 — corrections de l'audit 43 (navigateur), partie interface :
// réglages à liste par objet / pièce (P-a), vraies instances et dialogue
// « Définir le nombre d'instances » (F-a), sous-menus de formes des volumes
// (M1-a), interface utilisable jusqu'à 760 × 490 (U-a), Ctrl+G → .gcode.3mf
// (X-e), clic droit sur « Disposer » (S-a).
//
// Oracle Orca 2.4.1 : PRINT_CONFIG_CLASS_DEFINE(PrintObjectConfig /
// PrintRegionConfig) + SettingsFactory::get_options (GUI_Factories.cpp:163),
// Plater::increase_instances / set_number_of_copies (Plater.cpp:14656/14747),
// MenuFactory::append_submenu_add_generic (GUI_Factories.cpp:541),
// ObjectList::load_generic_subobject / create_mesh (GUI_ObjectList.cpp:2413),
// GUI_App::get_min_size (76 × 49 em), MainFrame.cpp:2786 (Ctrl+G).
import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { bootstrap, fixture, importModel } from "../lib/app";

type SceneDoc = {
  objects: {
    id: string;
    name?: string;
    settings?: Record<string, unknown>;
    instanceGroup?: string;
    volumes?: { type: string; name: string; shape: string; offset: number[]; modelId?: string }[];
  }[];
};

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
async function objectMenu(page: Page, row = 0) {
  await page.getByTestId("object-row").nth(row).click({ button: "right" });
  const menu = page.getByRole("menu");
  await expect(menu).toBeVisible();
  return menu;
}
async function positions(page: Page): Promise<number[][]> {
  const raw = (await page.getByTestId("scene-canvas").getAttribute("data-object-positions")) ?? "";
  return raw
    .split(";")
    .filter(Boolean)
    .map((p) => p.split(":")[1].split(",").map(Number));
}

test.describe("audit 43 (FR)", () => {
  test.use({ locale: "fr-FR" });

  // ─── P-a : réglages à liste par objet / pièce (+ X-e Ctrl+G) ───────────────
  test("P-a : listes de choix d'Orca en réglage d'objet (lissage) et de modificateur (motif), bordure réservée à l'objet ; tranché ; Ctrl+G → .gcode.3mf @F:editor.object-settings @F:ui.object-settings @F:editor.volume-modifier @F:menu.export-plate-sliced-file", async ({
    page,
  }) => {
    test.setTimeout(360_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("object-row").first().click({ button: "right" });
    await page.getByRole("menuitem", { name: /Ajouter des réglages/i }).click();
    const panel = page.getByTestId("object-settings-panel");
    await expect(panel).toBeVisible();
    // Clés enum absentes à l'audit 43 (27) : un échantillon région + objet.
    for (const key of [
      "ironing_type",
      "fuzzy_skin",
      "sparse_infill_pattern",
      "support_type",
      "seam_position",
      "brim_type",
      "wall_generator",
    ]) {
      await panel.getByTestId("object-settings-search").fill(key);
      await expect(panel.locator(`[data-testid="object-settings-result"][data-key="${key}"]`), key).toHaveCount(1);
    }
    await panel.getByTestId("object-settings-search").fill("ironing_type");
    await panel.locator('[data-testid="object-settings-result"][data-key="ironing_type"]').click();
    const line = panel.locator('[data-testid="object-setting"][data-key="ironing_type"]');
    await expect(line).toBeVisible();
    await line.locator("select").first().selectOption("top");

    // Modificateur : motif de remplissage proposé, bordure (PrintObjectConfig) non.
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.addModifier"]').click();
    const volPanel = page.getByTestId("volume-settings-panel");
    await volPanel.getByTestId("object-settings-add").click();
    await volPanel.getByTestId("object-settings-search").fill("sparse_infill_pattern");
    await expect(
      volPanel.locator('[data-testid="object-settings-result"][data-key="sparse_infill_pattern"]'),
    ).toHaveCount(1);
    await volPanel.getByTestId("object-settings-search").fill("brim_type");
    await expect(volPanel.locator('[data-testid="object-settings-result"][data-key="brim_type"]')).toHaveCount(0);

    await saveProject(page);
    const doc = await scene(page, id);
    expect(doc.objects[0].settings?.ironing_type).toBe("top");

    // Tranché : le lissage de l'objet apparaît dans le G-code (moteur Orca).
    await page.getByTestId("slice-button").click();
    await expect(page.getByTestId("export-gcode-button")).toBeEnabled({ timeout: 300_000 });
    const [gdl] = await Promise.all([page.waitForEvent("download"), page.getByTestId("export-gcode-button").click()]);
    expect(readFileSync((await gdl.path())!, "utf8")).toMatch(/;TYPE:Ironing/);
    // X-e : Ctrl+G = « Exporter le fichier tranché du plateau » → .gcode.3mf (MainFrame.cpp:2786).
    await page.locator("body").click({ position: { x: 5, y: 5 } });
    const [download] = await Promise.all([
      page.waitForEvent("download", { timeout: 60_000 }),
      page.keyboard.press("Control+g"),
    ]);
    expect(download.suggestedFilename()).toMatch(/\.gcode\.3mf$/);
    const bytes = readFileSync((await download.path())!);
    expect(bytes.includes(Buffer.from("Metadata/plate_1.gcode"))).toBe(true);
  });

  // ─── F-a : vraies instances, dialogue de l'application ──────────────────
  test("F-a : « Définir le nombre d'instances » — dialogue traduit, instances en escalier (5 % du plateau), réglages partagés, retour à 1 @F:ctx.set-number-of-instances @F:ctx.add-instance @F:ctx.remove-instance", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const nativeDialogs: string[] = [];
    page.on("dialog", (d) => {
      nativeDialogs.push(d.message());
      void d.dismiss();
    });
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.setInstances"]').click();
    const dialog = page.getByTestId("instance-count-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Copies de l'objet sélectionné");
    await expect(dialog).toContainText("Nombre de copies:");
    const input = dialog.getByTestId("instance-count-input");
    await expect(input).toHaveValue("1");
    await input.fill("1001");
    await expect(dialog.getByTestId("instance-count-ok")).toBeDisabled();
    await input.fill("3");
    await dialog.getByTestId("instance-count-ok").click();
    await expect(dialog).toHaveCount(0);
    expect(nativeDialogs).toEqual([]);
    const rows = page.getByTestId("object-row");
    await expect(rows).toHaveCount(3);
    await expect(page.getByTestId("object-name")).toHaveText([/cube20\.stl$/, /cube20\.stl$/, /cube20\.stl$/]);
    // Voron 350 : pas = 0,05 × 350 = 17,5 mm en X et Y depuis la dernière instance.
    const pos = await positions(page);
    expect(pos).toHaveLength(3);
    expect(pos[1][0] - pos[0][0]).toBeCloseTo(17.5, 1);
    expect(pos[1][1] - pos[0][1]).toBeCloseTo(17.5, 1);
    expect(pos[2][0] - pos[0][0]).toBeCloseTo(35, 1);

    // Réglage ajouté sur la 3ᵉ instance → partagé par les trois (même objet).
    await rows.nth(2).click({ button: "right" });
    await page.getByRole("menuitem", { name: /Ajouter des réglages/i }).click();
    const panel = page.getByTestId("object-settings-panel");
    await panel.getByTestId("object-settings-search").fill("wall_loops");
    await panel.locator('[data-testid="object-settings-result"][data-key="wall_loops"]').click();
    const wl = panel.locator('[data-testid="object-setting"][data-key="wall_loops"] input').first();
    await wl.fill("5");
    await wl.press("Tab");
    await saveProject(page);
    let doc = await scene(page, id);
    expect(doc.objects).toHaveLength(3);
    expect(doc.objects[0].instanceGroup).toBeTruthy();
    expect(new Set(doc.objects.map((o) => o.instanceGroup)).size).toBe(1);
    expect(doc.objects.map((o) => String(o.settings?.wall_loops))).toEqual(["5", "5", "5"]);

    // Rechargement : le groupe persiste ; « Retirer une instance » retire la dernière.
    await page.reload();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await expect(rows).toHaveCount(3, { timeout: 30_000 });
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.removeInstance"]').click();
    await expect(rows).toHaveCount(2);
    // Échap ferme le dialogue sans rien changer ; valeur initiale = instances actuelles.
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.setInstances"]').click();
    await expect(dialog.getByTestId("instance-count-input")).toHaveValue("2");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(rows).toHaveCount(2);
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.setInstances"]').click();
    await dialog.getByTestId("instance-count-input").fill("1");
    await dialog.getByTestId("instance-count-input").press("Enter");
    await expect(rows).toHaveCount(1);
    await saveProject(page);
    doc = await scene(page, id);
    expect(doc.objects).toHaveLength(1);
  });

  // ─── M1-a : sous-menus de formes ────────────────────────────────────────
  test("M1-a : « Ajouter un modificateur ▸ » Charger…, Cube… SVG ; Cylindre = maillage d'Orca « Générique-Cylindre » au coin avant droit ; Charger… en volume négatif @F:editor.volume-modifier @F:editor.volume-negative", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    const menu = await objectMenu(page);
    // Libellé = action d'origine (pavé) ; la flèche déplie les formes d'Orca.
    await page.getByTestId("menu-sub-object.addModifier").click();
    const shapes = page.getByTestId("menu-shapes-object.addModifier");
    await expect(shapes.getByRole("menuitem")).toHaveText([
      "Charger…",
      "Cube",
      "Cylindre",
      "Sphère",
      "Cône",
      "Disque",
      "Tore",
      "Texte",
      "SVG",
    ]);
    await shapes.getByRole("menuitem", { name: "Cylindre", exact: true }).click();
    await expect(menu).toHaveCount(0);
    await expect(page.getByText(/Générique-Cylindre/).first()).toBeVisible({ timeout: 30_000 });

    // Charger… : fichier importé comme volume négatif, nommé d'après le fichier.
    await objectMenu(page);
    await page.getByTestId("menu-sub-object.addNegativeVolume").click();
    const [chooser] = await Promise.all([
      page.waitForEvent("filechooser"),
      page.getByTestId("menu-shapes-object.addNegativeVolume").getByRole("menuitem", { name: "Charger…" }).click(),
    ]);
    await chooser.setFiles(fixture("cylinder"));
    await expect(page.getByText("cylinder.stl").first()).toBeVisible({ timeout: 60_000 });

    await saveProject(page);
    const doc = await scene(page, id);
    const vols = doc.objects[0].volumes ?? [];
    const cyl = vols.find((v) => v.name === "Générique-Cylindre");
    expect(cyl?.type).toBe("modifier");
    expect(cyl?.shape).toBe("mesh");
    expect(cyl?.modelId).toBeTruthy();
    // Orca create_mesh : côté = 0,1 × 350 = 35 mm ; centre = (max.x, min.y, min.z) + ½ côté
    // de la boîte du cube de 20 mm, dans son repère recentré → (27,5 ; 7,5 ; 7,5).
    expect(cyl!.offset[0]).toBeCloseTo(27.5, 0);
    expect(cyl!.offset[1]).toBeCloseTo(7.5, 0);
    expect(cyl!.offset[2]).toBeCloseTo(7.5, 0);
    const neg = vols.find((v) => v.name === "cylinder.stl");
    expect(neg?.type).toBe("negative");
    expect(neg?.shape).toBe("mesh");
  });

  // ─── S-a : clic droit sur « Disposer » ──────────────────────────────────
  test("S-a : clic droit sur « Disposer » → options d'arrangement, jamais le menu de la scène @F:toolbar.arrange", async ({
    page,
  }) => {
    await ready(page, ["cube20"]);
    await page.getByTestId("toolbar-arrange").click({ button: "right" });
    await expect(page.getByRole("menu")).toHaveCount(0);
    await expect(page.getByTestId("arrange-options-panel")).toBeVisible();
  });

  // ─── U-a : petites fenêtres ─────────────────────────────────────────────
  for (const [w, h] of [
    [1024, 640],
    [760, 490],
  ]) {
    test(`U-a : ${w}×${h} — onglets et actions atteignables, page sans défilement, rail d'outils défilant @F:core.slice-ui`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: w, height: h });
      await ready(page, ["cube20"]);
      const reachable = (testid: string | null, name: string | null) =>
        page.evaluate(
          ([testid, name]) => {
            const el = testid
              ? document.querySelector(`[data-testid="${testid}"]`)
              : [...document.querySelectorAll('[data-testid="editor-topbar"] nav button')].find((b) =>
                  (b.textContent ?? "").trim().startsWith(name!),
                );
            if (!el) return "absent";
            const r = el.getBoundingClientRect();
            const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
            return hit && (hit === el || el.contains(hit))
              ? "ok"
              : `couvert par ${(hit?.textContent ?? "").trim().slice(0, 20)}`;
          },
          [testid, name] as const,
        );
      for (const tab of ["Préparer", "Aperçu", "Appareil", "Projet"])
        expect(await reachable(null, tab), tab).toBe("ok");
      for (const id of ["slice-button", "save-project", "export-gcode-button", "plate-settings-button"])
        expect(await reachable(id, null), id).toBe("ok");
      const doc = await page.evaluate(() => ({
        w: document.documentElement.scrollWidth,
        h: document.documentElement.scrollHeight,
      }));
      expect(doc).toEqual({ w, h });
      // Le rail d'outils défile dans sa colonne : le dernier outil reste atteignable.
      const last = page.locator('aside[aria-label="Outils"] .ws-tool').last();
      await last.scrollIntoViewIfNeeded();
      await expect(last).toBeInViewport();
      // Onglet Aperçu cliquable à la souris.
      const preview = page.getByTestId("editor-topbar").locator("nav button", { hasText: "Aperçu" });
      await preview.click();
      await expect(preview).toHaveAttribute("aria-current", "page");
    });
  }
});

test.describe("audit 43 (EN)", () => {
  test.use({ locale: "en-US" });
  test("F-a : instance dialog translated (Copies of the selected object) @F:ctx.set-number-of-instances", async ({
    page,
  }) => {
    await page.addInitScript(() => localStorage.setItem("PARAGLIDE_LOCALE", "en"));
    await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objects", exact: true }).click();
    await objectMenu(page);
    await page.locator('[role=menu] [data-action="object.setInstances"]').click();
    const dialog = page.getByTestId("instance-count-dialog");
    await expect(dialog).toContainText("Copies of the selected object");
    await expect(dialog).toContainText("Number of copies:");
    await expect(dialog.getByTestId("instance-count-cancel")).toHaveText("Cancel");
  });
});
