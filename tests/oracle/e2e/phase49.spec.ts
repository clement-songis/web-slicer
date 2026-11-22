// Phase 49 — correctifs de l'audit 47 (navigateur), partie interface :
// ouverture d'un projet d'une autre imprimante (PROJ-a : sélecteurs du 3MF),
// plages « autres couches » affichées = enregistrées (PLATE-b, ordre réel des
// filaments dans le G-code), avertissement « surface irrégulière désactivée »
// + lien d'activation (FUZZ-a), plus de Forcer/Bloquer sur cet outil (FUZZ-b),
// touches N et L des outils de peinture (KEY-a).
//
// Oracle Orca 2.4.1 : Plater::load_files / PresetBundle::load_config_file_config
// (presets du projet sélectionnés), OtherLayersSeqPanel::append_layer +
// LayerSeqInfo::operator< + LayerNumberTextInput::validate_input_value
// (PlateSettingsDialog.cpp:276-300), GLGizmoFuzzySkin.cpp:326-357 (avertissement,
// lien → FuzzySkinType::None, PrintConfig.cpp:3540 enum « none » = « Painted only »),
// GLGizmoMmuSegmentation.cpp:90 (WXK_CONTROL_N) et GLGizmoFdmSupports.cpp:81
// (WXK_CONTROL_L) via GLGizmosManager::handle_shortcut.
// I18N-b (nom de plaque traduit, Print.cpp:1771, OrcaSlicer_fr.po:10922) est
// couvert par la spec phase48 (I18N-a) et validation-p49.test.ts.
// Menu Vue : « Show Wireframe » est commenté dans MainFrame.cpp:3139-3142
// d'Orca 2.4.1 — pas d'entrée à ajouter.
//
// CONTRAT: printer-picker[data-project-printer], preset-select-print,
// preset-select-filament(-N), plate-layer-range(-begin/-end-kind/-end/-order),
// paint-state (absent en canal fuzzy), paint-fuzzy-disabled, paint-fuzzy-enable,
// tool-mm-paint, tool-support-paint, tool-fuzzy-paint, scene-canvas,
// export-gcode-button.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, fixture, importModel, uniqueEmail } from "../lib/app";

type Obj = { settings?: Record<string, unknown> };
type SceneDoc = {
  objects: Obj[];
  plates?: { plates?: { settings?: Record<string, unknown> }[] };
};

async function ready(page: Page, models: string[]): Promise<string> {
  const id = await bootstrap(page);
  for (const m of models) {
    await importModel(page, m);
    await expect(page.getByTestId("import-status")).toHaveCount(0, {
      timeout: 60_000,
    });
  }
  return id;
}

async function bootstrapWith(
  page: Page,
  printer: string,
  filament: string,
  process: string,
): Promise<string> {
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

async function selectFirst(page: Page) {
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("object-name").first().click();
}

/** Amène l'ordre d'un FilamentOrderEditor (boutons « plus tôt ») à `target`. */
async function orderTo(
  scope: ReturnType<Page["getByTestId"]>,
  target: number[],
  testid: string,
) {
  for (let guard = 0; guard < 12; guard++) {
    const cur = await scope
      .getByTestId(testid)
      .evaluate((el) =>
        [...el.querySelectorAll("[data-filament]")].map((x) =>
          Number(x.getAttribute("data-filament")),
        ),
      );
    if (cur.join(",") === target.join(",")) return;
    for (let i = 0; i < cur.length; i++) {
      if (cur[i] !== target[i]) {
        await scope
          .getByTestId(testid)
          .locator(`[aria-label*="filament ${target[i]}"]`)
          .first()
          .click();
        break;
      }
    }
    await scope.page().waitForTimeout(100);
  }
  throw new Error(`ordre non atteint : ${target.join(",")}`);
}

test.describe("audit 47 (FR)", () => {
  test("PROJ-a : « Ouvrir un projet » d'une autre imprimante — les sélecteurs montrent l'imprimante, le traitement et les filaments du 3MF @F:menu.open-project @F:param.printer", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await bootstrapWith(
      page,
      "Bambu Lab P1S 0.4 nozzle",
      "Bambu PLA Basic @BBL X1C",
      "0.20mm Standard @BBL X1C",
    );
    // 3MF Voron (imprimante non possédée) : Fichier › Ouvrir un projet.
    await page
      .getByTestId("project-open-input")
      .setInputFiles(fixture("scenes/mmu_painted.3mf"));
    await expect(page.getByTestId("scene-canvas")).toBeVisible({
      timeout: 60_000,
    });
    await page.waitForFunction(
      () => !document.querySelector("[data-testid=import-status]"),
      null,
      { timeout: 120_000 },
    );
    // Imprimante du 3MF affichée telle quelle (Plater::load_files), sans
    // activer une imprimante possédée.
    const printer = page.getByTestId("printer-picker");
    await expect(printer).toBeVisible({ timeout: 60_000 });
    await expect(printer).toHaveAttribute("data-project-printer", "true");
    await expect(printer).toHaveValue("");
    await expect(printer.locator("option").first()).toHaveText(
      "Voron 2.4 350 0.4 nozzle",
    );
    // Traitement du 3MF sélectionné (preset importé « (configuration 3MF) »).
    const process = page.getByTestId("preset-select-print");
    await expect(process).toBeVisible({ timeout: 60_000 });
    await expect(process).not.toHaveValue("");
    expect(await process.evaluate((s) => s.selectedOptions[0]?.text)).toMatch(
      /mmu_painted.*configuration 3MF/,
    );
    // Filaments du 3MF (2 slots), pas ceux du projet précédent.
    await expect(page.getByTestId("preset-select-filament")).not.toHaveValue(
      "",
    );
    await expect(page.getByTestId("preset-select-filament-2")).not.toHaveValue(
      "",
    );
  });

  test("PLATE-b : plages « autres couches » — valeurs affichées = valeurs enregistrées, ordre des filaments du G-code par plage @F:editor.plate-settings @F:editor.multi-filament", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    await ready(page, ["cube20", "cylinder", "bridge", "overhang"]);
    // 4 filaments, 4 objets sur les extrudeurs 1-4 (tour d'essuyage : l'ordre
    // par couche est observable dans le G-code — scénario de l'audit 47).
    await expect(page.getByTestId("preset-select-filament")).toBeVisible();
    for (let i = 0; i < 3; i++)
      await page.getByTestId("add-filament-slot").click();
    await expect(page.getByTestId("filament-swatch")).toHaveCount(4);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    const rows = page.getByTestId("object-row");
    for (let i = 0; i < 4; i++) {
      await rows
        .nth(i)
        .locator(
          'select[aria-label*="Extrudeur"], select[aria-label*="Extruder"]',
        )
        .first()
        .selectOption(String(i + 1));
    }
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
    // 1ʳᵉ couche : 4,3,2,1.
    await orderTo(
      dialog.getByTestId("plate-first-layer-sequence"),
      [4, 3, 2, 1],
      "plate-first-layer-order",
    );
    // Plage 1 (2 → Fin) : fin personnalisée 4, ordre 2,1,3,4.
    const range = (i: number) => dialog.getByTestId("plate-layer-range").nth(i);
    await range(0)
      .getByTestId("plate-layer-range-end-kind")
      .selectOption("custom");
    await range(0).getByTestId("plate-layer-range-end").fill("4");
    await range(0).getByTestId("plate-layer-range-end").press("Tab");
    await orderTo(range(0), [2, 1, 3, 4], "plate-layer-range-order");
    // Plage 2 (ajoutée vide) : début 5, fin 9, ordre 4,1,2,3 — la saisie
    // reste sur SA ligne (identité stable des lignes à la clé range.id).
    await range(1).getByTestId("plate-layer-range-begin").fill("5");
    await range(1).getByTestId("plate-layer-range-begin").press("Tab");
    await range(1)
      .getByTestId("plate-layer-range-end-kind")
      .selectOption("custom");
    await range(1).getByTestId("plate-layer-range-end").fill("9");
    await range(1).getByTestId("plate-layer-range-end").press("Tab");
    await orderTo(range(1), [4, 1, 2, 3], "plate-layer-range-order");
    // Plage 3 : début 10 → Fin, ordre 4,3,2,1.
    await range(2).getByTestId("plate-layer-range-begin").fill("10");
    await range(2).getByTestId("plate-layer-range-begin").press("Tab");
    await orderTo(range(2), [4, 3, 2, 1], "plate-layer-range-order");
    // Affiché : 2→4, 5→9, 10→Fin avec leurs ordres.
    const displayed = await dialog
      .getByTestId("plate-layer-range")
      .evaluateAll((rs) =>
        rs.map((r) => {
          const b = r.querySelector(
            "[data-testid=plate-layer-range-begin]",
          ) as HTMLInputElement | null;
          const k = r.querySelector(
            "[data-testid=plate-layer-range-end-kind]",
          ) as HTMLSelectElement | null;
          const e = r.querySelector(
            "[data-testid=plate-layer-range-end]",
          ) as HTMLInputElement | null;
          const o = r.querySelector("[data-testid=plate-layer-range-order]");
          return (
            `${b?.value || "∅"}→${e?.value || (k?.value === "end" ? "Fin" : "∅")} [` +
            (o
              ? [...o.querySelectorAll("[data-filament]")]
                  .map((x) => x.getAttribute("data-filament"))
                  .join(",")
              : "?") +
            "]"
          );
        }),
      );
    expect(displayed).toEqual([
      "2→4 [2,1,3,4]",
      "5→9 [4,1,2,3]",
      "10→Fin [4,3,2,1]",
    ]);
    await dialog.getByTestId("settings-apply").click();
    await expect(dialog).toHaveCount(0);
    await page.getByTestId("save-project").click();
    await expect(page.getByTestId("save-project")).toBeEnabled();
    const projectId = new URL(page.url()).pathname.split("/").pop()!;
    // Enregistré : exactement les valeurs affichées (format d'Orca,
    // OtherLayersSeqPanel à l'enregistrement ; « Fin » = INT_MAX − 1).
    const id = projectId;
    await expect
      .poll(
        async () =>
          (await scene(page, id)).plates?.plates?.[0]?.settings
            ?.other_layers_print_sequence,
        { timeout: 30_000 },
      )
      .toBe("2,4,2,1,3,4,5,9,4,1,2,3,10,2147483646,4,3,2,1");
    expect(
      (await scene(page, id)).plates?.plates?.[0]?.settings
        ?.other_layers_print_sequence_nums,
    ).toBe("3");
    expect(
      (await scene(page, id)).plates?.plates?.[0]?.settings
        ?.first_layer_print_sequence,
    ).toBe("4,3,2,1");
    // Ordre réel des filaments dans le G-code (T n par couche, filament 1 = T0) :
    // couches 2-4 → 2,1,3,4 ; 5-9 → 4,1,2,3 ; 10+ → 4,3,2,1 (le premier T d'une
    // couche peut précéder le marqueur ; la répartition de référence vient de
    // la CLI Orca sur le même 3MF — cf. rapport phase 49).
    await page.getByTestId("slice-button").click();
    await expect(page.getByTestId("preview-view")).toHaveAttribute(
      "data-layer-count",
      /^[1-9]\d*$/,
      { timeout: 240_000 },
    );
    const [dl] = await Promise.all([
      page.waitForEvent("download", { timeout: 60_000 }),
      page.getByTestId("export-gcode-button").click(),
    ]);
    const gcode = (await dl.path())!;
    const { readFileSync } = await import("node:fs");
    const text = readFileSync(gcode, "utf8");
    const layers: string[][] = [];
    for (const line of text.split("\n")) {
      if (line.startsWith(";LAYER_CHANGE")) layers.push([]);
      const t = line.match(/^T(\d+)\s*$/);
      if (t && layers.length) layers[layers.length - 1].push(t[1]);
    }
    expect(layers.length).toBeGreaterThanOrEqual(12);
    const seq = (i: number) => layers[i]?.join("");
    expect(seq(1)).toBe("1023");
    expect(seq(2)).toBe("1023");
    expect(seq(3)).toBe("1023");
    // Couche 5 : le T3 d'ouverture est émis avant le LAYER_CHANGE.
    expect(seq(4)).toBe("012");
    expect(seq(5)).toBe("3012");
    expect(seq(8)).toBe("3012");
    expect(seq(9)).toBe("3210");
    expect(seq(11)).toBe("3210");
  });

  test("KEY-a : N ouvre la peinture couleur, L la peinture de supports (touches simples d'Orca) @F:shortcut.n @F:shortcut.l @F:gizmo.mmsegmentation @F:gizmo.fdmsupports", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await ready(page, ["cube20"]);
    await selectFirst(page);
    const canvas = page.getByTestId("scene-canvas");
    const box = (await canvas.boundingBox())!;
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await page.keyboard.press("n");
    await expect(page.getByTestId("tool-mm-paint")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(page.getByTestId("paint-tool-hint")).toBeVisible();
    await page.keyboard.press("Escape");
    await page.keyboard.press("l");
    await expect(page.getByTestId("tool-support-paint")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await page.keyboard.press("Escape");
  });

  test("FUZZ-a/b : surface irrégulière désactivée — avertissement + lien « Activer… » (l'objet passe en « Peint uniquement »), pas de choix Forcer/Bloquer @F:gizmo.fuzzyskin", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await ready(page, ["cube20"]);
    await selectFirst(page);
    const canvas = page.getByTestId("scene-canvas");
    const box = (await canvas.boundingBox())!;
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    // Traitement par défaut : fuzzy_skin = « Désactivé » (défaut Orca).
    await page.getByTestId("tool-fuzzy-paint").click();
    await expect(page.getByTestId("paint-tool-hint")).toBeVisible();
    // FUZZ-b : la surface irrégulière n'a qu'un état peint (aucun Forcer/Bloquer).
    await expect(page.getByTestId("paint-state")).toHaveCount(0);
    // FUZZ-a : avertissement d'Orca (GLGizmoFuzzySkin.cpp:337) + lien (ibid. 343).
    const warn = page.getByTestId("paint-fuzzy-disabled");
    await expect(warn).toBeVisible();
    await expect(warn).toContainText(
      "Attention : La surface irrégulière est désactivée, la surface irrégulière peinte ne prendra pas effet !",
    );
    await page.getByTestId("paint-fuzzy-enable").click();
    await expect(warn).toHaveCount(0);
    // Le lien passe l'objet en « Peint uniquement » (FuzzySkinType::None).
    await page.keyboard.press("Escape");
    const projectId = new URL(page.url()).pathname.split("/").pop()!;
    await page.getByTestId("save-project").click();
    await expect
      .poll(
        async () =>
          (await scene(page, projectId)).objects?.[0]?.settings?.fuzzy_skin,
        { timeout: 30_000 },
      )
      .toBe("none");
  });
});
