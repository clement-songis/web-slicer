// Phase 28 — lot 26 branché dans l'interface : découpe complète d'Orca (plan
// incliné, connecteurs posés sur la face de coupe, queue d'aronde, pièces),
// vignette du plateau envoyée avant l'export 3MF, texte en relief ré-éditable,
// frictions de la phase 27 (imports côte à côte, séparateur décimal, infobulle).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

async function openCut(page: Page) {
  await bootstrap(page);
  await importModel(page, "cube20");
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-cut").click();
  await expect(page.getByTestId("cut-tool")).toBeVisible();
}

function cutPost(page: Page) {
  return page.waitForResponse(
    (r) => r.request().method() === "POST" && /\/cut$/.test(r.url()),
    {
      timeout: 120_000,
    },
  );
}

test("coupe inclinée : angles du plan → plan dessiné incliné, rotation envoyée", async ({
  page,
}) => {
  await openCut(page);
  const canvas = page.getByTestId("scene-canvas");
  await page.getByTestId("cut-rot-x").fill("30");
  await page.getByTestId("cut-rot-x").press("Enter");
  await expect(canvas).toHaveAttribute(
    "data-guide-normal",
    "0.000,-0.500,0.866",
  );
  await expect(page.getByTestId("cut-tilted")).toBeVisible();
  const [res] = await Promise.all([
    cutPost(page),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(201);
  const body = res.request().postDataJSON();
  expect(body.rotation_deg).toEqual([30, 0, 0]);
  expect(body.plane.normal[2]).toBeCloseTo(0.866, 3);
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  // Pièces posées à côté l'une de l'autre : pas d'avis de superposition.
  await expect(page.getByTestId("overlap-arrange")).toHaveCount(0);
});

test("connecteurs : clic sur la face de coupe, goujon carré, volumes renvoyés posés dans la scène", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await openCut(page);
  const canvas = page.getByTestId("scene-canvas");
  await page.getByTestId("cut-connectors-edit").click();
  await expect(canvas).toHaveAttribute("data-guide-pickable", "true");
  const map = page.getByTestId("cut-section-map");
  const box = (await map.boundingBox())!;
  await page.getByTestId("cut-type-dowel").click();
  await page.getByTestId("cut-shape-square").click();
  await page.mouse.click(box.x + box.width / 2 - 20, box.y + box.height / 2);
  await page.mouse.click(box.x + box.width / 2 + 20, box.y + box.height / 2);
  await expect(page.getByTestId("cut-connector-count")).toHaveText("2");
  await expect(canvas).toHaveAttribute("data-guide-connectors", "2");
  // Avec des connecteurs, les deux moitiés sont gardées et « pièces » est indisponible (Orca).
  await expect(page.getByTestId("cut-keep-lower")).toBeDisabled();
  await expect(page.getByTestId("cut-as-parts")).toBeDisabled();
  const [res] = await Promise.all([
    cutPost(page),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(201);
  const body = res.request().postDataJSON();
  expect(body.connectors).toHaveLength(2);
  expect(body.connectors[0]).toMatchObject({
    type: "dowel",
    shape: "square",
    style: "prism",
  });
  expect(body.connectors[0].pos[2]).toBeCloseTo(0, 6);
  const parts = (await res.json()).parts as {
    label: string;
    volumes?: unknown[];
  }[];
  expect(parts.some((p) => (p.volumes?.length ?? 0) > 0)).toBe(true);
  await expect(page.getByTestId("editor-message")).toContainText(
    "2 connecteur(s)",
  );
  await expect(page.getByTestId("object-row")).toHaveCount(parts.length);
});

test("queue d’aronde et « couper en pièces »", async ({ page }) => {
  test.setTimeout(240_000);
  await openCut(page);
  await page.getByTestId("cut-mode-groove").click();
  await expect(page.getByTestId("cut-groove")).toBeVisible();
  await expect(page.getByTestId("cut-as-parts")).toBeDisabled();
  const [res] = await Promise.all([
    cutPost(page),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(201);
  expect(res.request().postDataJSON().groove).toMatchObject({
    flaps_angle_deg: 60,
    count: 1,
  });
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  // Couper en pièces : un seul objet (2e moitié = volume).
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-cut").click();
  // Le mode choisi est gardé d'une ouverture à l'autre (m_mode d'Orca).
  await expect(page.getByTestId("cut-mode-groove")).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await page.getByTestId("cut-mode-planar").click();
  await page.getByTestId("cut-as-parts").check();
  await expect(page.getByTestId("cut-place-upper")).toBeDisabled();
  const [res2] = await Promise.all([
    cutPost(page),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res2.status(), await res2.text()).toBe(201);
  expect(res2.request().postDataJSON().keep_as_parts).toBe(true);
  await expect(page.getByTestId("object-row")).toHaveCount(2);
});

test("export 3MF : capture du plateau envoyée avant l’export, présente dans le fichier", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  // Sauvegarde préalable à l'export : refusée tant que l'import est en cours.
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  const put = page.waitForResponse(
    (r) =>
      r.request().method() === "PUT" && /\/plates\/0\/thumbnail$/.test(r.url()),
    { timeout: 60_000 },
  );
  const download = page.waitForEvent("download", { timeout: 60_000 });
  await page.locator('a[href$="/export/3mf"]').click();
  expect((await put).status()).toBe(204);
  const file = await (await download).path();
  const { readFileSync } = await import("node:fs");
  const bytes = readFileSync(file!).toString("latin1");
  expect(bytes).toContain("Metadata/plate_1.png");
  expect(bytes).toContain("Metadata/plate_1_small.png");
});

test("texte en relief ré-éditable : double-clic et menu « Modifier le texte »", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-emboss").click();
  await page.getByTestId("emboss-text").fill("Web");
  await page.getByTestId("emboss-submit").click();
  await expect(page.getByTestId("editor-message")).toContainText(
    "ajoutée sur",
    { timeout: 120_000 },
  );
  // Menu contextuel de l'objet : « Modifier le texte » rouvre l'outil avec le texte.
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Modifier le texte" }).click();
  await expect(page.getByTestId("emboss-editing")).toBeVisible();
  await expect(page.getByTestId("emboss-text")).toHaveValue("Web");
  await page.getByTestId("emboss-text").fill("Ok");
  await page.getByTestId("emboss-submit").click();
  await expect(page.getByTestId("editor-message")).toContainText(
    "relief mis à jour",
    {
      timeout: 120_000,
    },
  );
  // Double-clic dans la vue : même chose, texte mis à jour relu.
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
  const [x, y] = (await canvas.getAttribute("data-object-anchor"))!
    .split(",")
    .map(Number);
  const box = (await canvas.boundingBox())!;
  await page.mouse.dblclick(box.x + x, box.y + y);
  await expect(page.getByTestId("emboss-text")).toHaveValue("Ok");
});

test("frictions : imports côte à côte, séparateur décimal homogène, infobulle traduite", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  await page.waitForTimeout(500);
  await expect(page.getByTestId("overlap-arrange")).toHaveCount(0);
  await page.getByRole("button", { name: "Traitement", exact: true }).click();
  await page.getByTestId("open-settings-print").click();
  const dlg = page.getByTestId("settings-dialog");
  await dlg.getByTestId("settings-mode-develop").click();
  await dlg.getByRole("tab", { name: /Qualité/ }).click();
  const values = await dlg
    .locator("input[type=text]")
    .evaluateAll((es) => es.map((e) => (e as HTMLInputElement).value));
  expect(values.some((v) => /^\d+,\d+/.test(v))).toBe(true);
  expect(values.filter((v) => /^\d+\.\d+/.test(v))).toEqual([]);
  await page.keyboard.press("Escape");
  await page.getByTestId("open-settings-printer").click();
  await dlg.getByTestId("settings-mode-develop").click();
  const found = await (async () => {
    for (const tab of await dlg.locator("[role=tab]").all()) {
      await tab.click();
      const html = await dlg.innerHTML();
      if (html.includes("mise en forme du signal dans le firmware"))
        return html;
    }
    return "";
  })();
  expect(found).toContain("« Défaut »");
  expect(found).not.toContain("« Default »");
});
