// Éditeur : découpes serveur (worker moteur `mesh-op`, libslic3r) — séparation
// en objets (`TriangleMesh::split`), coupe plane horizontale (`cut_mesh`) et
// booléen mcut. Parcours UI réels contre la pile : menu / outil → endpoint
// `/api/models/{id}/{split,cut}` ou `/api/projects/{id}/boolean` → nouvelles
// pièces dans la liste d'objets, source retirée (comme Orca).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap } from "../lib/app";

type Box = { min: [number, number, number]; size: number };

/** STL binaire (normales réelles, en-tête ≠ « solid ») de cubes alignés. */
function cubesStl(cubes: Box[]): Buffer {
  const tris: number[][][] = [];
  for (const { min, size } of cubes) {
    const p = (i: number) => [
      min[0] + (i & 1 ? size : 0),
      min[1] + (i & 2 ? size : 0),
      min[2] + (i & 4 ? size : 0),
    ];
    for (const [a, b, c, d] of [
      [0, 2, 3, 1],
      [4, 5, 7, 6],
      [0, 1, 5, 4],
      [2, 6, 7, 3],
      [0, 4, 6, 2],
      [1, 3, 7, 5],
    ]) {
      tris.push([p(a), p(b), p(c)], [p(a), p(c), p(d)]);
    }
  }
  const buf = Buffer.alloc(84 + tris.length * 50);
  buf.write("e2e cubes", 0);
  buf.writeUInt32LE(tris.length, 80);
  tris.forEach(([a, b, c], i) => {
    const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    const n = [
      u[1] * w[2] - u[2] * w[1],
      u[2] * w[0] - u[0] * w[2],
      u[0] * w[1] - u[1] * w[0],
    ];
    const len = Math.hypot(n[0], n[1], n[2]);
    const o = 84 + i * 50;
    [n[0] / len, n[1] / len, n[2] / len, ...a, ...b, ...c].forEach((v, k) =>
      buf.writeFloatLE(v, o + k * 4),
    );
  });
  return buf;
}

async function importStl(page: Page, name: string, cubes: Box[]) {
  const before = await page.getByTestId("object-row").count();
  await page
    .getByTestId("import-input")
    .setInputFiles({ name, mimeType: "model/stl", buffer: cubesStl(cubes) });
  await expect(page.getByTestId("object-row")).toHaveCount(before + 1, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

/** Bouton visible du rail d'outils (le rail existe aussi en copie masquée). */
function railTool(page: Page, name: RegExp) {
  return page
    .getByRole("toolbar", { name: "Outils" })
    .getByRole("button", { name })
    .filter({ visible: true })
    .first();
}

type Part = { label: string; volume_mm3: number; watertight: boolean };

test("séparer en objets : deux cubes disjoints → deux objets @F:editor.mesh-split", async ({
  page,
}) => {
  await bootstrap(page);
  await importStl(page, "pair.stl", [
    { min: [0, 0, 0], size: 10 },
    { min: [20, 0, 0], size: 10 },
  ]);
  await page.getByTestId("object-row").first().click({ button: "right" });
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/split$/.test(r.url()),
    ),
    page.getByRole("menuitem", { name: /to objects|en objets/i }).click(),
  ]);
  expect(res.status()).toBe(201);
  const parts = ((await res.json()) as { parts: Part[] }).parts;
  expect(parts).toHaveLength(2);
  for (const part of parts) {
    expect(part.volume_mm3).toBeCloseTo(1000, 1);
    expect(part.watertight).toBe(true);
  }
  // Source retirée, deux pièces ajoutées.
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  await expect(page.getByTestId("object-row")).toContainText([
    "pair_1",
    "pair_2",
  ]);
});

test("coupe horizontale : cube de 20 mm coupé à +2,5 mm @F:editor.mesh-cut", async ({
  page,
}) => {
  await bootstrap(page);
  await importStl(page, "block.stl", [{ min: [0, 0, 0], size: 20 }]);
  await page.getByTestId("object-row").first().click();
  await railTool(page, /^(cut|coupe)$/i).click();
  await expect(page.getByTestId("cut-apply")).toBeVisible();
  // Hauteur depuis le plateau (phase 26) : milieu du bloc (10) + 2,5 mm.
  await page.getByTestId("cut-offset-input").fill("12.5");
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/cut$/.test(r.url()),
    ),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res.status()).toBe(201);
  const body = res.request().postDataJSON() as {
    plane: { point: number[]; normal: number[] };
  };
  expect(body.plane.normal).toEqual([0, 0, 1]);
  expect(body.plane.point[2]).toBeCloseTo(2.5, 6);
  const parts = ((await res.json()) as { parts: Part[] }).parts;
  const byLabel = Object.fromEntries(parts.map((p) => [p.label, p]));
  expect(byLabel.upper.volume_mm3).toBeCloseTo(3000, 0);
  expect(byLabel.lower.volume_mm3).toBeCloseTo(5000, 0);
  expect(byLabel.upper.watertight && byLabel.lower.watertight).toBe(true);
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  await expect(page.getByTestId("object-row")).toContainText([
    "block_A",
    "block_B",
  ]);
});

test("booléen : différence A − B (B traverse la face supérieure de A) @F:editor.mesh-boolean", async ({
  page,
}) => {
  await bootstrap(page);
  // Importés au centre du plateau, posés à z = 0 (fonds coplanaires = cas
  // dégénéré de mcut). On relève B (10 mm) à z ∈ [13, 23] : il traverse la face
  // supérieure de A (20 mm) — mcut n'opère qu'où les surfaces se coupent.
  // A − B = 8000 − 10×10×7 = 7300 mm³.
  await importStl(page, "outer.stl", [{ min: [0, 0, 0], size: 20 }]);
  await importStl(page, "inner.stl", [{ min: [0, 0, 0], size: 10 }]);
  const rows = page.getByTestId("object-row");
  // Phase 28 : le 2e import est posé à côté du 1er (Orca, cellule libre) — on
  // recentre B sur A avant de le relever.
  await rows.nth(0).click();
  await railTool(page, /flatten|poser/i).click();
  const ax = await page.getByLabel(/Position \(mm\) x/i).inputValue();
  const ay = await page.getByLabel(/Position \(mm\) y/i).inputValue();
  await rows.nth(1).click();
  await page.getByLabel(/Position \(mm\) x/i).fill(ax);
  await page.getByLabel(/Position \(mm\) y/i).fill(ay);
  const z = page.getByLabel(/Position \(mm\) z/i);
  await expect(z).toHaveValue(/^5(\.0+)?$/);
  await z.fill("18");
  await rows.nth(0).click();
  await rows.nth(1).click({ modifiers: ["Control"] });
  await railTool(page, /boolean|booléen/i).click();
  await page.getByTestId("boolean-op").selectOption("difference");
  await expect(page.getByTestId("boolean-apply")).toBeEnabled();
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/boolean$/.test(r.url()),
    ),
    page.getByTestId("boolean-apply").click(),
  ]);
  expect(res.status()).toBe(201);
  const parts = ((await res.json()) as { parts: Part[] }).parts;
  expect(parts).toHaveLength(1);
  expect(parts[0].volume_mm3).toBeCloseTo(7300, -1);
  // Différence : A consommé, outil B conservé + résultat.
  await expect(rows).toHaveCount(2);
  await expect(rows).toContainText(["inner", "outer_boolean"]);
});

test("séparation refusée (une seule pièce) : erreur affichée @F:editor.mesh-split", async ({
  page,
}) => {
  await bootstrap(page);
  await importStl(page, "single.stl", [{ min: [0, 0, 0], size: 10 }]);
  await page.getByTestId("object-row").first().click({ button: "right" });
  const [res] = await Promise.all([
    page.waitForResponse((r) => /\/split$/.test(r.url())),
    page.getByRole("menuitem", { name: /to objects|en objets/i }).click(),
  ]);
  expect(res.status()).toBe(422);
  await expect(page.getByText(/Séparation impossible/)).toBeVisible();
  await expect(page.getByTestId("object-row")).toHaveCount(1);
});
