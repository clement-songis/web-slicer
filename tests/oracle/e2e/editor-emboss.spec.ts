// Outil Emboss (texte en relief, Emboss::text2shapes + polygons2model côté
// moteur) : l'objet créé apparaît dans la liste et se tranche.
import { expect, test } from "@playwright/test";
import { bootstrap, importModel, sliceInUi } from "../lib/app";

test("Emboss › texte en relief : objet créé et tranché @F:editor.emboss-text", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const projectId = await bootstrap(page);
  const fonts = await (await page.request.get("/api/emboss/fonts")).json();
  expect(fonts.length, "polices embarquées disponibles").toBeGreaterThan(0);

  // Route directe : contrat de l'API.
  const res = await page.request.post(`/api/projects/${projectId}/emboss`, {
    data: { text: "Orca", font: fonts[0].id, size_mm: 10, depth_mm: 2 },
  });
  expect(res.status()).toBe(201);
  const part = (await res.json()).parts[0];
  expect(part.triangles).toBeGreaterThan(100);
  expect(part.watertight).toBe(true);
  const bad = await page.request.post(`/api/projects/${projectId}/emboss`, {
    data: { text: "  ", font: fonts[0].id, size_mm: 10, depth_mm: 2 },
  });
  expect(bad.status()).toBe(422);

  // Parcours UI : outil Emboss → objet ajouté → tranchage.
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  }); // modèle créé par l'API
  await page.getByTestId("tool-emboss").click();
  await page.getByTestId("emboss-text").fill("Web");
  await page.getByTestId("emboss-submit").click();
  await expect(page.getByTestId("object-row")).toHaveCount(2, {
    timeout: 120_000,
  });
  await sliceInUi(page);
});

test("Emboss › forme SVG mise à l’échelle et graisse du texte @F:editor.emboss-svg", async ({
  page,
}) => {
  const projectId = await bootstrap(page);
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20">' +
    '<path fill="black" fill-rule="evenodd" d="M0,0 H40 V20 H0 Z M10,5 H20 V15 H10 Z"/></svg>';
  const res = await page.request.post(`/api/projects/${projectId}/emboss-svg`, {
    data: { svg, width_mm: 30, depth_mm: 2 },
  });
  expect(res.status()).toBe(201);
  const part = (await res.json()).parts[0];
  // Deux contours fermés evenodd : le trou doit survivre à l'extrusion.
  expect(part.triangles).toBe(32);
  expect(part.volume_mm3).toBeGreaterThan(0);
  expect(part.volume_mm3).toBeLessThan(900);
  const bad = await page.request.post(`/api/projects/${projectId}/emboss-svg`, {
    data: { svg: "pas un svg", width_mm: 30, depth_mm: 2 },
  });
  expect(bad.status()).toBe(422);

  const fonts = await (await page.request.get("/api/emboss/fonts")).json();
  const thin = await (
    await page.request.post(`/api/projects/${projectId}/emboss`, {
      data: { text: "I", font: fonts[0].id, size_mm: 20, depth_mm: 2 },
    })
  ).json();
  const bold = await (
    await page.request.post(`/api/projects/${projectId}/emboss`, {
      data: {
        text: "I",
        font: fonts[0].id,
        size_mm: 20,
        depth_mm: 2,
        boldness_mm: 1,
      },
    })
  ).json();
  expect(bold.parts[0].volume_mm3).toBeGreaterThan(thin.parts[0].volume_mm3);
});

test("Emboss › texte gravé en creux dans l’objet sélectionné @F:editor.emboss-engrave", async ({
  page,
}) => {
  test.setTimeout(300_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  const rows = page.getByTestId("object-row");
  await expect(rows).toHaveCount(1);
  await page.getByTestId("object-name").first().click();

  await page.getByTestId("tool-emboss").click();
  await page.getByTestId("emboss-text").fill("W");
  await page.getByTestId("emboss-raised").uncheck();
  // Profondeur de gravure 1 mm (outil de 2 mm centré sur la face supérieure).
  await page.getByTestId("emboss-panel-depth").fill("1");
  const [boolRes] = await Promise.all([
    page.waitForResponse(
      (r) => r.url().includes("/boolean") && r.request().method() === "POST",
      {
        timeout: 180_000,
      },
    ),
    page.getByTestId("emboss-submit").click(),
  ]);
  expect(boolRes.status()).toBe(201);
  const part = (await boolRes.json()).parts[0];
  expect(part.volume_mm3).toBeLessThan(8000);
  expect(part.volume_mm3).toBeGreaterThan(7000);

  // Le résultat conserve le nom du modèle A ; une seule ligne prouve le remplacement.
  await expect(rows).toHaveCount(1, { timeout: 60_000 });
  await expect(page.getByTestId("object-name").first()).toContainText("cube20");
});
