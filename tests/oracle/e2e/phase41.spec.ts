// Phase 41 — D1 : réglages modifiés sans enregistrer de preset (preset
// « (modifié) » d'Orca) persistés avec le projet (lot 40,
// `active_presets.overrides`) : sauvegarde, rechargement, export 3MF
// (project_settings.config), G-code, réouverture du 3MF, « Abandonner ».
// A3 : plus de sonde 422 volontaire (/validate { scene: 0 }) à l'ouverture.
//
// Oracle Orca 2.4.1 : un preset modifié non enregistré fait partie du projet
// (PresetBundle::full_config → Metadata/project_settings.config, bbs_3mf.cpp),
// relu à l'ouverture (load_project → preset sélectionné + « (modifié) »,
// PresetComboBox::update suffixe « (modified) ») ; UnsavedChangesDialog
// « Abandonner » oublie les modifications du preset quitté.
import { expect, test, type Page } from "@playwright/test";
import { inflateRawSync } from "node:zlib";
import {
  bootstrap,
  downloadGcode,
  importModel,
  openParam,
  oracleCfg,
  sliceInUi,
} from "../lib/app";
import { configBlock } from "../lib/gcode";

type Project = {
  id: string;
  active_presets: {
    process?: string;
    overrides?: { process?: Record<string, unknown> };
  } | null;
};

/** Entrées d'une archive ZIP (3MF), sans dépendance. */
function unzip(buf: Buffer): Record<string, Buffer> {
  let eocd = buf.length - 22;
  while (eocd >= 0 && buf.readUInt32LE(eocd) !== 0x06054b50) eocd--;
  const out: Record<string, Buffer> = {};
  let p = buf.readUInt32LE(eocd + 16);
  for (let i = 0, n = buf.readUInt16LE(eocd + 10); i < n; i++) {
    const method = buf.readUInt16LE(p + 10),
      size = buf.readUInt32LE(p + 20);
    const nl = buf.readUInt16LE(p + 28),
      el = buf.readUInt16LE(p + 30),
      cl = buf.readUInt16LE(p + 32);
    const off = buf.readUInt32LE(p + 42);
    const name = buf.toString("utf8", p + 46, p + 46 + nl);
    const start =
      off + 30 + buf.readUInt16LE(off + 26) + buf.readUInt16LE(off + 28);
    const data = buf.subarray(start, start + size);
    out[name] = method === 8 ? inflateRawSync(data) : Buffer.from(data);
    p += 46 + nl + el + cl;
  }
  return out;
}

async function getProject(page: Page, id: string): Promise<Project> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as Project;
}

/** « Sauvegarder » (barre supérieure) : attend le PUT du projet. */
async function saveProject(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
  await expect(page.getByTestId("save-project")).toHaveAttribute(
    "data-unsaved",
    "false",
  );
}

/** wall_loops passé à 5 dans le dialogue Process, sans enregistrer le preset. */
async function editWallLoops(page: Page, value: string) {
  const control = await openParam(page, "print", "Strength", "wall_loops");
  await expect(control).toBeEnabled({ timeout: 30_000 });
  await expect(control).not.toHaveValue("");
  const before = await control.inputValue();
  expect(before, "wall_loops du preset déjà égal à la valeur de test").not.toBe(
    value,
  );
  await control.fill(value);
  await control.press("Tab");
  const dialog = page.getByTestId("settings-dialog");
  await expect(dialog.getByTestId("preset-modified")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  return before;
}

/** Badge « (modifié) » du preset Process + valeur 5 dans le dialogue. */
async function expectModifiedWallLoops(page: Page, value: string) {
  await expect(page.getByTestId("preset-select-print")).toHaveAttribute(
    "data-modified",
    "true",
    {
      timeout: 30_000,
    },
  );
  await expect(page.getByTestId("preset-select-print-modified")).toBeVisible();
  const control = await openParam(page, "print", "Strength", "wall_loops");
  await expect(control).toHaveValue(value, { timeout: 30_000 });
  await expect(
    page.getByTestId("settings-dialog").getByTestId("preset-modified"),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
}

test.describe("phase 41 (FR)", () => {
  test.use({ locale: "fr-FR" });

  test("A3 : ouverture de l’éditeur sans sonde 422 ni erreur console @F:core.slice-ui", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    const errors: string[] = [];
    const rejected: string[] = [];
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    page.on("response", (r) => {
      if (r.url().includes("/api/") && r.status() === 422)
        rejected.push(
          `${r.request().method()} ${r.url()} ${r.request().postData() ?? ""}`,
        );
    });
    await bootstrap(page);
    // Validation en direct (scène dans le corps si non enregistrée, lot 29),
    // sans sondage préalable.
    const validated = page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/validate$/.test(r.url()),
      { timeout: 90_000 },
    );
    await importModel(page, "cube20");
    await expect(page.getByTestId("import-status")).toHaveCount(0, {
      timeout: 60_000,
    });
    const res = await validated;
    expect(res.status()).toBe(200);
    expect(
      JSON.parse(res.request().postData() ?? "{}").scene,
      "sonde { scene: 0 }",
    ).not.toBe(0);
    await page.waitForTimeout(1_500);
    expect(rejected, "réponses 422 à l’ouverture").toEqual([]);
    expect(
      errors.filter((e) => /422|Unprocessable/i.test(e)),
      "erreurs console 422",
    ).toEqual([]);
  });

  test("D1 : wall_loops 3→5 sans enregistrer de preset → sauvegarde, rechargement, export 3MF, G-code @F:core.project-save @F:ui.preset-save-as @F:core.export-gcode", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    const id = await bootstrap(page);
    await importModel(page, "cube20");
    await expect(page.getByTestId("import-status")).toHaveCount(0, {
      timeout: 60_000,
    });
    const processId = (await getProject(page, id)).active_presets?.process;
    // « 0.20mm Standard @Voron » : wall_loops = 3 (fdm_process_voron_common).
    expect(await editWallLoops(page, "5")).toBe("3");
    await expect(
      page.getByTestId("preset-select-print-modified"),
    ).toBeVisible();
    // Un réglage modifié rend le projet « non enregistré ».
    await expect(page.getByTestId("save-project")).toHaveAttribute(
      "data-unsaved",
      "true",
    );
    await saveProject(page);

    // Persisté dans le projet, sans preset enregistré : même preset, surcharge.
    const saved = await getProject(page, id);
    expect(saved.active_presets?.process).toBe(processId);
    expect(Number(saved.active_presets?.overrides?.process?.wall_loops)).toBe(
      5,
    );
    expect(Object.keys(saved.active_presets?.overrides?.process ?? {})).toEqual(
      ["wall_loops"],
    );

    // Rechargement : valeur 5 restaurée, badge visible, projet « enregistré ».
    await page.reload();
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expectModifiedWallLoops(page, "5");
    await expect(page.getByTestId("save-project")).toHaveAttribute(
      "data-unsaved",
      "false",
    );

    // Export 3MF : config complète du projet (Orca full_config).
    const tmf = await page.request.get(`/api/projects/${id}/export/3mf`);
    expect(tmf.status()).toBe(200);
    const entries = unzip(Buffer.from(await tmf.body()));
    const settings = JSON.parse(
      entries["Metadata/project_settings.config"].toString("utf8"),
    ) as Record<string, unknown>;
    expect(String(settings.wall_loops)).toBe("5");

    // G-code : en-tête de config (« ; wall_loops = 5 »).
    await sliceInUi(page);
    const gcode = await downloadGcode(page);
    expect(gcode).toContain("; wall_loops = 5");
    expect(configBlock(gcode).wall_loops).toBe("5");

    // Réouverture du 3MF exporté (comme un 3MF Orca) : surcharge relue.
    const imp = await page.request.post("/api/projects/import", {
      headers: { origin: new URL(page.url()).origin },
      multipart: {
        file: {
          name: "phase41.3mf",
          mimeType: "model/3mf",
          buffer: Buffer.from(await tmf.body()),
        },
      },
    });
    expect(imp.status(), await imp.text()).toBeLessThan(300);
    const imported = (await imp.json()) as Project;
    expect(
      Number(imported.active_presets?.overrides?.process?.wall_loops),
    ).toBe(5);
    await page.goto(`/projects/${imported.id}`);
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expectModifiedWallLoops(page, "5");
  });

  test("D1 : « Abandonner » au changement de preset retire la surcharge du projet @F:ui.preset-management @F:core.project-save", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    const id = await bootstrap(page);
    await importModel(page, "cube20");
    await expect(page.getByTestId("import-status")).toHaveCount(0, {
      timeout: 60_000,
    });
    // Second preset process (dérivé du preset système, comme « Enregistrer sous »).
    const list = (await (
      await page.request.get("/api/presets?kind=process")
    ).json()) as {
      id: string;
      name: string;
    }[];
    const parent = list.find((p) => p.name === oracleCfg.process)!;
    const name = `P41 autre process ${Date.now().toString(36)}`;
    const created = await page.request.post("/api/presets", {
      data: {
        kind: "process",
        name,
        inherits: parent.id,
        values: { layer_height: 0.24 },
      },
    });
    expect(created.status(), await created.text()).toBe(201);
    const otherId = ((await created.json()) as { id: string }).id;

    await editWallLoops(page, "5");
    await saveProject(page);
    await page.reload();
    await expectModifiedWallLoops(page, "5");

    await page.getByTestId("preset-select-print").selectOption(otherId);
    const unsaved = page.getByTestId("unsaved-preset-dialog");
    await expect(
      unsaved.locator("[data-testid=unsaved-row][data-key=wall_loops]"),
    ).toBeVisible();
    await unsaved.getByTestId("unsaved-discard").click();
    await expect(page.getByTestId("preset-select-print")).toHaveValue(otherId);
    await expect(page.getByTestId("preset-select-print-modified")).toHaveCount(
      0,
    );
    await saveProject(page);
    const after = await getProject(page, id);
    expect(after.active_presets?.process).toBe(otherId);
    expect(after.active_presets?.overrides?.process).toBeUndefined();

    // Rechargement : plus de « (modifié) ».
    await page.reload();
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(page.getByTestId("preset-select-print")).toHaveAttribute(
      "data-modified",
      "false",
    );
  });
});
