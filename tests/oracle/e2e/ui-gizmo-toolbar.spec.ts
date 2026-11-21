import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel, openParam } from "../lib/app";

type SceneObject = {
  id: string;
  modelId?: string;
  transform: { position: number[]; rotation: number[]; scale: number[] };
  settings?: Record<string, unknown>;
  painting?: Record<string, Record<string, string>>;
  layerProfile?: number[];
  brimEars?: { points: number[][] };
  volumes?: {
    id: string;
    type: string;
    size: number[];
    offset: number[];
    settings: Record<string, unknown>;
  }[];
};
type Project = {
  id: string;
  version: number;
  scene: { objects: SceneObject[] };
  active_presets: { process?: string; printer?: string; filament?: string[] };
};

async function project(page: Page, id: string): Promise<Project> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return res.json();
}

async function save(page: Page, id: string): Promise<Project> {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        r.url().endsWith(`/api/projects/${id}`),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
  return project(page, id);
}

async function ready(page: Page) {
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

async function setup(page: Page, models = ["cube20"]): Promise<string> {
  const id = await bootstrap(page);
  for (const model of models) await importModel(page, model);
  await ready(page);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  return id;
}

async function select(page: Page, index = 0) {
  // Le nom (et non le centre de la ligne, qui peut tomber sur un contrôle).
  await page.getByTestId("object-name").nth(index).click();
}

async function clickObject(page: Page, dx = 0) {
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
  const [x, y] = (await canvas.getAttribute("data-object-anchor"))!
    .split(",")
    .map(Number);
  const box = (await canvas.boundingBox())!;
  await page.mouse.click(box.x + x + dx, box.y + y);
}

function cubesStl(): Buffer {
  // Même générateur que editor-split.spec.ts : normales réelles, en-tête ≠ « solid ».
  const tris: number[][][] = [];
  for (const x0 of [0, 20]) {
    const p = (i: number) => [
      x0 + (i & 1 ? 10 : 0),
      i & 2 ? 10 : 0,
      i & 4 ? 10 : 0,
    ];
    for (const [a, b, c, d] of [
      [0, 2, 3, 1],
      [4, 5, 7, 6],
      [0, 1, 5, 4],
      [2, 6, 7, 3],
      [0, 4, 6, 2],
      [1, 3, 7, 5],
    ])
      tris.push([p(a), p(b), p(c)], [p(a), p(c), p(d)]);
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
    [n[0] / len, n[1] / len, n[2] / len, ...a, ...b, ...c].forEach((v, k) =>
      buf.writeFloatLE(v, 84 + i * 50 + k * 4),
    );
  });
  return buf;
}

test("gizmos Move/Rotate/Scale : transformations visibles et persistées @F:gizmo.move @F:gizmo.rotate @F:gizmo.scale", async ({
  page,
}) => {
  const id = await setup(page);
  await select(page);
  const x = page.getByLabel("Position (mm) X", { exact: true });
  const before = Number(await x.inputValue());
  await x.fill(String(before + 7));
  await page.getByTestId("tool-rotate").click();
  await page.getByLabel("Rotation (°) Z", { exact: true }).fill("30");
  await page.getByTestId("tool-scale").click();
  await page.getByLabel("Échelle X", { exact: true }).fill("150");
  const saved = await save(page, id);
  const t = saved.scene.objects[0].transform;
  expect(t.position[0]).toBeCloseTo(before + 7, 5);
  expect(t.rotation[2]).toBeCloseTo(30, 5);
  expect(t.scale[0]).toBeCloseTo(1.5, 5);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await select(page);
  await expect(page.getByLabel("Position (mm) X", { exact: true })).toHaveValue(
    String(before + 7),
  );
});

test("Flatten : faces candidates, clic sur une face → posée à plat @F:gizmo.flatten", async ({
  page,
}) => {
  const id = await setup(page);
  await select(page);
  await page.getByTestId("tool-rotate").click();
  await page.getByLabel("Rotation (°) X", { exact: true }).fill("30");
  await page.getByTestId("tool-flatten").click();
  const canvas = page.getByTestId("scene-canvas");
  // Cube : 6 plans d'enveloppe convexe (GLGizmoFlatten::update_planes).
  await expect(canvas).toHaveAttribute("data-flatten-planes", "6");
  const anchors = (await canvas.getAttribute("data-flatten-anchors"))!
    .split(";")
    .map((a) => a.split(",").map(Number));
  const box = (await canvas.boundingBox())!;
  // Face la plus haute à l'écran (dessus incliné), pas celle déjà en bas.
  const [ax, ay] = anchors.reduce((best, a) => (a[1] < best[1] ? a : best));
  await page.mouse.move(box.x + ax, box.y + ay);
  await page.mouse.click(box.x + ax, box.y + ay);
  const rx = page.getByLabel("Rotation (°) X", { exact: true });
  await expect
    .poll(async () => Number(await rx.inputValue()))
    .not.toBeCloseTo(30, 3);
  const saved = await save(page, id);
  const t = saved.scene.objects[0].transform;
  // Face posée : chaque angle est un multiple de 90°, cube reposant à z = 10.
  for (const a of t.rotation)
    expect(Math.abs(Math.sin((a * Math.PI) / 90))).toBeLessThan(1e-4);
  expect(t.position[2]).toBeCloseTo(10, 3);
  // Raccourci conservé : « Poser à plat » remet la rotation à zéro.
  await page.getByRole("button", { name: "Poser à plat" }).click();
  await expect(rx).toHaveValue("0");
});
test("Cut : le plan produit deux objets volumétriques @F:gizmo.cut", async ({
  page,
}) => {
  await setup(page);
  await select(page);
  await page.getByTestId("tool-cut").click();
  // Hauteur depuis le plateau (phase 26) : milieu du cube (10) + 2,5 mm.
  await page.getByTestId("cut-offset-input").fill("12.5");
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/cut$/.test(r.url()),
    ),
    page.getByTestId("cut-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(201);
  const parts = (await res.json()).parts;
  expect(
    parts
      .map((p: { volume_mm3: number }) => p.volume_mm3)
      .sort((a: number, b: number) => a - b),
  ).toEqual([3000, 5000]);
  await expect(page.getByTestId("object-row")).toHaveCount(2);
});

test("Mesh Boolean : différence A−B appliquée au maillage @F:gizmo.meshboolean", async ({
  page,
}) => {
  await setup(page, ["cube20", "cube20"]);
  // Phase 28 : le 2e import est posé à côté (Orca) — on recentre B sur A.
  await select(page, 0);
  await page.getByTestId("tool-flatten").click();
  const ax = await page
    .getByLabel("Position (mm) X", { exact: true })
    .inputValue();
  const ay = await page
    .getByLabel("Position (mm) Y", { exact: true })
    .inputValue();
  await select(page, 1);
  await page.getByLabel("Position (mm) X", { exact: true }).fill(ax);
  await page.getByLabel("Position (mm) Y", { exact: true }).fill(ay);
  await page.getByLabel("Position (mm) Z", { exact: true }).fill("18");
  await select(page, 0);
  await page
    .getByTestId("object-row")
    .nth(1)
    .click({ modifiers: ["Control"] });
  await page.getByTestId("tool-boolean").click();
  await page.getByTestId("boolean-op").selectOption("difference");
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/boolean$/.test(r.url()),
    ),
    page.getByTestId("boolean-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(201);
  expect((await res.json()).parts[0].volume_mm3).toBeCloseTo(3200, -1); // 8000 − 20×20×12 (B : z ∈ [8, 28])
  await expect(page.getByTestId("object-row")).toHaveCount(2);
});

test("peinture Supports/Seam/Fuzzy/MMU : facettes sérialisées au format Orca @F:gizmo.fdmsupports @F:gizmo.seam @F:gizmo.fuzzyskin @F:gizmo.mmsegmentation", async ({
  page,
}) => {
  const id = await setup(page);
  await select(page);
  for (const tool of [
    "support-paint",
    "seam-paint",
    "fuzzy-paint",
    "mm-paint",
  ]) {
    await page.getByTestId(`tool-${tool}`).click();
    await page.getByTestId("paint-tool-fill").click();
    await clickObject(page);
    await expect(
      page.getByText(/^[1-9]\d* triangle\(s\) peint\(s\)$/),
    ).toBeVisible();
  }
  const saved = await save(page, id);
  const painting = saved.scene.objects[0].painting ?? {};
  for (const channel of ["supports", "seam", "fuzzy", "mmu"])
    expect(Object.keys(painting[channel] ?? {}).length).toBeGreaterThan(0);
});

test("Emboss et SVG : création de deux maillages depuis le rail @F:gizmo.emboss @F:gizmo.svg", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await bootstrap(page);
  await page.getByTestId("tool-emboss").click();
  await page.getByTestId("emboss-text").fill("Web");
  await page.getByTestId("emboss-submit").click();
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 120_000,
  });
  // Sans sélection : la forme SVG devient un nouvel objet (avec un objet
  // sélectionné, elle serait posée dessus comme pièce — phase 26).
  await page.keyboard.press("Escape");
  await page.getByTestId("tool-svg").click();
  await page.getByText("Source SVG (coller)").click();
  await page
    .getByTestId("emboss-svg")
    .fill(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0H10V10H0Z"/></svg>',
    );
  await page.getByTestId("emboss-submit").click();
  await expect(page.getByTestId("object-row")).toHaveCount(2, {
    timeout: 120_000,
  });
});

test("Measure et BrimEars : picking 3D, distance et point persisté @F:gizmo.measure @F:gizmo.brimears", async ({
  page,
}) => {
  const id = await setup(page);
  await select(page);
  await page.getByTestId("tool-measure").click();
  await clickObject(page, -8);
  await clickObject(page, 8);
  await expect(page.getByTestId("measure-distance")).not.toHaveText("0.00 mm");
  await page.getByTestId("tool-brim-ears").click();
  await clickObject(page);
  await expect(
    page.getByRole("button", { name: "Retirer l'oreille 1" }),
  ).toBeVisible();
  const saved = await save(page, id);
  expect(saved.scene.objects[0].brimEars?.points).toHaveLength(1);
});

test("Assembly View et hauteur variable : éclaté visuel et profil persisté @F:gizmo.assembly @F:toolbar.assembly-view @F:toolbar.layersediting", async ({
  page,
}) => {
  const id = await setup(page, ["cube20", "cube20"]);
  const canvas = page.getByTestId("scene-canvas");
  await page.getByTestId("toolbar-arrange").click();
  await page.getByTestId("arrange-all").click();
  await expect(page.getByTestId("editor-message")).toContainText("Arrangement");
  const before = await canvas.getAttribute("data-object-positions");
  await page.getByTestId("toolbar-assembly_view").click();
  await expect(page.getByTestId("toolbar-assembly_view")).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.getByRole("checkbox", { name: "Vue éclatée" }).check();
  await expect(canvas).not.toHaveAttribute("data-object-positions", before!);
  await select(page, 0);
  await page.getByTestId("toolbar-layersediting").click();
  await page.getByLabel("Z de", { exact: true }).fill("0");
  await page.getByLabel("Z à", { exact: true }).fill("5");
  await page.getByLabel("Hauteur", { exact: true }).fill("0.12");
  await page.getByRole("button", { name: "Appliquer la bande" }).click();
  const saved = await save(page, id);
  // Profil posé sur l'objet sélectionné (ordre de sauvegarde indépendant de la liste).
  expect(saved.scene.objects.flatMap((o) => o.layerProfile ?? [])).toContain(
    0.12,
  );
});

test("barre Add/Add plate/Add instance/Remove instance @F:toolbar.add @F:toolbar.addplate @F:toolbar.more @F:toolbar.fewer", async ({
  page,
}) => {
  await bootstrap(page);
  await page.getByTestId("toolbar-add").click();
  await page.getByRole("menuitem", { name: "Cube", exact: true }).click();
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await ready(page);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  // La primitive ajoutée peut déjà être sélectionnée : un clic la désélectionnerait.
  const more = page.getByTestId("toolbar-more");
  if (await more.isDisabled()) await select(page);
  await expect(more).toBeEnabled();
  await more.click();
  await expect(page.getByTestId("object-row")).toHaveCount(2);
  const fewer = page.getByTestId("toolbar-fewer");
  if (await fewer.isDisabled()) await select(page, 1);
  await expect(fewer).toBeEnabled();
  await fewer.click();
  await expect(page.getByTestId("object-row")).toHaveCount(1);
  await page.getByTestId("toolbar-addplate").click();
  await expect(page.getByTestId("plate-tab")).toHaveCount(2);
});

test("Auto orient et Arrange : rotation et placements moteur appliqués @F:toolbar.orient @F:toolbar.arrange", async ({
  page,
}) => {
  const id = await setup(page, ["cube20", "cube20"]);
  await select(page);
  await page.getByTestId("tool-rotate").click();
  await page.getByLabel("Rotation (°) Z", { exact: true }).fill("30");
  const [orient] = await Promise.all([
    page.waitForResponse((r) => /\/orient$/.test(r.url())),
    page.getByTestId("toolbar-orient").click(),
  ]);
  expect(orient.status()).toBe(200);
  await expect(page.getByLabel("Rotation (°) Z", { exact: true })).toHaveValue(
    "0",
  );
  // Phase 35 : le bouton ouvre le menu d'options ; « Tout arranger » dispose.
  await page.getByTestId("toolbar-arrange").click();
  const [arrange] = await Promise.all([
    page.waitForResponse((r) => /\/arrange$/.test(r.url())),
    page.getByTestId("arrange-all").click(),
  ]);
  expect(arrange.status()).toBe(200);
  const saved = await save(page, id);
  const [a, b] = saved.scene.objects.map((o) => o.transform.position);
  expect(Math.hypot(a[0] - b[0], a[1] - b[1])).toBeGreaterThan(20);
});

test("Split to objects : deux composants deviennent deux objets @F:toolbar.splitobjects", async ({
  page,
}) => {
  await bootstrap(page);
  await page.getByTestId("import-input").setInputFiles({
    name: "pair.stl",
    mimeType: "model/stl",
    buffer: cubesStl(),
  });
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await ready(page);
  await select(page);
  const [res] = await Promise.all([
    page.waitForResponse((r) => /\/split$/.test(r.url())),
    page.getByTestId("toolbar-splitobjects").click(),
  ]);
  expect(res.status()).toBe(201);
  await expect(page.getByTestId("object-row")).toHaveCount(2);
});

test("Simplify : réduction du maillage visible et persistée @F:gizmo.simplify", async ({
  page,
}) => {
  await setup(page, ["cylinder"]);
  await select(page);
  await page.getByTestId("tool-simplify").click();
  await page.getByTestId("simplify-mode-ratio").check();
  await page.getByTestId("simplify-ratio").fill("50");
  const [res] = await Promise.all([
    page.waitForResponse((r) => /\/simplify$/.test(r.url())),
    page.getByTestId("simplify-apply").click(),
  ]);
  expect(res.status(), await res.text()).toBe(200);
  const body = await res.json();
  expect(body.triangles_after).toBeLessThan(body.triangles_before);
  await expect(page.getByTestId("simplify-tool")).toContainText(
    `Maillage actuel : ${body.triangles_after} triangles`,
  );
});

test("réglages objet et modificateur de plage Z persistés @F:ui.object-settings @F:ui.height-range-modifier @F:ctx.height-range-modifier", async ({
  page,
}) => {
  const id = await setup(page);
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page
    .getByRole("menuitem", { name: /Add settings|Ajouter des réglages/i })
    .click();
  const panel = page.getByTestId("object-settings-panel");
  await panel.getByTestId("object-settings-search").fill("wall_loops");
  await panel
    .locator('[data-testid="object-settings-result"][data-key="wall_loops"]')
    .click();
  const objLine = panel
    .locator('[data-testid="object-setting"][data-key="wall_loops"]')
    .locator("input")
    .first();
  await objLine.fill("5");
  await objLine.press("Tab");
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: /Height range Modifier|Modificateur de plage de hauteur/i }).click();
  // Orca layer_config_ranges : première plage [0, 2] avec layer_height courant.
  const range = page.getByTestId("layer-range");
  await expect(range).toHaveCount(1);
  await page.getByLabel("Z max de la plage 1", { exact: true }).fill("5");
  await page.getByLabel("Z max de la plage 1", { exact: true }).press("Tab");
  const height = range
    .locator('[data-testid="object-setting"][data-key="layer_height"]')
    .locator("input")
    .first();
  await height.fill("0.12");
  await height.press("Tab");
  // « + » : plage suivante [5, 7].
  await page
    .getByRole("button", { name: "Ajouter une plage après la plage 1" })
    .click();
  await expect(range).toHaveCount(2);
  const saved = await save(page, id);
  expect(String(saved.scene.objects[0].settings?.wall_loops)).toBe("5");
  const ranges = (
    saved.scene.objects[0] as {
      layerRanges?: {
        min: number;
        max: number;
        settings: Record<string, unknown>;
      }[];
    }
  ).layerRanges!;
  expect(ranges.map((r) => [r.min, r.max])).toEqual([
    [0, 5],
    [5, 7],
  ]);
  expect(Number(ranges[0].settings.layer_height)).toBeCloseTo(0.12, 6);
  expect(saved.scene.objects[0].volumes ?? []).toHaveLength(0);
});

test("preset système modifié : copie utilisateur réutilisable, parent intact @F:ui.preset-save-as", async ({
  page,
}) => {
  test.setTimeout(240_000);
  const id = await bootstrap(page);
  const initial = await project(page, id);
  // Le bootstrap sélectionne le preset process système de oracle.json.
  const originalId = initial.active_presets.process!;
  const original = await (
    await page.request.get(`/api/presets/${originalId}`)
  ).json();
  expect(original.origin).toBe("system");
  const control = await openParam(
    page,
    "print",
    "Strength",
    "wall_loops",
    "develop",
  );
  await control.fill("5");
  await control.press("Tab");
  await expect(page.getByTestId("preset-select-print")).toHaveAttribute(
    "data-modified",
    "true",
  );
  const [created] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === "POST" && /\/api\/presets$/.test(r.url()),
      { timeout: 60_000 },
    ),
    page.getByTestId("settings-apply").click(),
  ]);
  expect(created.status(), await created.text()).toBe(201);
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
  const saved = await project(page, id);
  const copyId = saved.active_presets.process!;
  expect(copyId).not.toBe(originalId);
  const copy = await (await page.request.get(`/api/presets/${copyId}`)).json();
  expect(copy.origin).toBe("user");
  expect(
    copy.values.wall_loops,
    `corps du POST : ${JSON.stringify(created.request().postDataJSON())}`,
  ).toBe(5);
  const after = await (
    await page.request.get(`/api/presets/${originalId}`)
  ).json();
  expect(after.values).toEqual(original.values);
  const second = await page.request.post("/api/projects", {
    data: {
      name: "Réutilisation preset",
      active_presets: saved.active_presets,
    },
  });
  expect(second.status()).toBe(201);
  const secondId = (await second.json()).id;
  await page.goto(`/projects/${secondId}`);
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await expect(page.getByTestId("preset-select-print")).toHaveValue(copyId);
});

test("Split to parts : une seule ligne, volume pièce maillé persisté @F:toolbar.splitvolumes", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await page.getByTestId("import-input").setInputFiles({
    name: "pair.stl",
    mimeType: "model/stl",
    buffer: cubesStl(),
  });
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await ready(page);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await select(page);
  const [res] = await Promise.all([
    page.waitForResponse((r) => /\/split$/.test(r.url())),
    page.getByTestId("toolbar-splitvolumes").click(),
  ]);
  expect(res.status()).toBe(201);
  const parts = (await res.json()).parts as {
    model: { id: string };
    center: number[];
  }[];
  expect(parts).toHaveLength(2);
  // Orca split_volume : un seul objet, la 2e composante devient un volume pièce
  // nommé comme dans `ModelVolume::split` (`<nom>_2`) ; l'objet garde son nom.
  await expect(page.getByTestId("object-row")).toHaveCount(1);
  await expect(page.getByTestId("object-name")).toContainText("pair.stl");
  await expect(page.getByTestId("object-name")).not.toContainText("pair.stl_");
  // Arborescence Orca après split : pair.stl › pair.stl_1 (maillage de l'objet), pair.stl_2.
  await expect(page.getByTestId("object-volume")).toHaveCount(2);
  await expect(page.getByTestId("object-volume")).toHaveText([
    /pair\.stl_1/,
    /pair\.stl_2/,
  ]);
  await expect(
    page.locator('[data-testid="object-volume"][data-type="part"]'),
  ).toHaveCount(2);
  const saved = await save(page, id);
  expect(saved.scene.objects).toHaveLength(1);
  const object = saved.scene.objects[0] as SceneObject & {
    volumes: {
      shape: string;
      modelId: string;
      offset: number[];
      type: string;
      name: string;
    }[];
  };
  expect(object.modelId).toBe(parts[0].model.id);
  expect((object as { meshName?: string }).meshName).toBe("pair.stl_1");
  expect(object.volumes).toHaveLength(1);
  expect(object.volumes[0]).toMatchObject({
    type: "part",
    shape: "mesh",
    modelId: parts[1].model.id,
    name: "pair.stl_2",
  });
  expect(Math.abs(object.volumes[0].offset[0])).toBeCloseTo(20, 3);
  // Rechargement : le volume maillé est restauré.
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await select(page);
  await expect(page.getByTestId("object-volume")).toHaveText([
    /pair\.stl_1/,
    /pair\.stl_2/,
  ]);
});

/** Sélectionne une autre ligne et attend que l'ancre écran de l'objet change. */
async function selectOther(page: Page, index: number) {
  const canvas = page.getByTestId("scene-canvas");
  const previous = await canvas.getAttribute("data-object-anchor");
  await select(page, index);
  if (previous)
    await expect(canvas).not.toHaveAttribute("data-object-anchor", previous);
}
test("Assembly : point à point puis face à face rapprochent l’objet mobile @F:gizmo.assembly", async ({
  page,
}) => {
  const id = await setup(page, ["cube20", "cube20"]);
  // (Espacement 8 mm pour écarter franchement les deux cubes avant l'assemblage.)
  await page.getByTestId("toolbar-arrange").click();
  await page.getByTestId("arrange-spacing-input").fill("8");
  await page.getByTestId("arrange-all").click();
  await expect(page.getByTestId("editor-message")).toContainText("Arrangement");
  const before = await save(page, id);
  const dist = (p: Project) => {
    const [a, b] = p.scene.objects.map((o) => o.transform.position);
    return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
  };
  expect(dist(before)).toBeGreaterThan(25);
  await select(page, 0);
  await page.getByTestId("tool-assembly").click();
  await page.getByTestId("assembly-mode").selectOption("point");
  await clickObject(page);
  await expect(page.getByTestId("assembly-hint")).toHaveAttribute(
    "data-picks",
    "1",
  );
  await selectOther(page, 1);
  await clickObject(page);
  await expect(page.getByTestId("assembly-hint")).toHaveAttribute(
    "data-picks",
    "0",
  );
  const pointed = await save(page, id);
  // Deux cubes identiques, points cliqués au centre apparent : centres très proches.
  expect(dist(pointed)).toBeLessThan(8);

  // Face à face : la face cliquée de B vient contre celle de A (normales opposées).
  await page.getByTestId("toolbar-arrange").click();
  await page.getByTestId("arrange-all").click();
  await expect(page.getByTestId("editor-message")).toContainText("Arrangement");
  await page.getByTestId("assembly-mode").selectOption("face");
  await select(page, 0);
  await clickObject(page);
  await selectOther(page, 1);
  await clickObject(page);
  await expect(page.getByTestId("editor-message")).toContainText(
    "Assemblage appliqué",
  );
  const faced = await save(page, id);
  // Faces en contact puis reposée sur le plateau (auto_drop d'Orca, GLCanvas3D
  // do_rotate/do_move) : B rejoint A et est retourné.
  expect(dist(faced)).toBeLessThan(dist(pointed) + 1);
  expect(faced.scene.objects[1].transform.rotation).not.toEqual(
    pointed.scene.objects[1].transform.rotation,
  );
});
