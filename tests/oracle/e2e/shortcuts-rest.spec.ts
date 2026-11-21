import { expect, test, type Page } from "@playwright/test";
import { bootstrap, fixture, importModel, sliceInUi } from "../lib/app";

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

async function setup(page: Page, models: string[] = ["cube20"]) {
  await bootstrap(page);
  for (const model of models) await importModel(page, model);
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("object-row")).toHaveCount(models.length);
  await page.getByRole("button", { name: "Objets", exact: true }).click();
}

async function positions(page: Page) {
  const text = await page
    .getByTestId("scene-canvas")
    .getAttribute("data-object-positions");
  return (text ?? "")
    .split(";")
    .filter(Boolean)
    .map((entry) => {
      const [id, xyz] = entry.split(":");
      return { id, xyz: xyz.split(",").map(Number) };
    });
}

test("projet : ouvrir, sauvegarder, sauvegarder sous, importer et préférences @F:shortcut.ctrl-o @F:shortcut.ctrl-s @F:shortcut.ctrl-shift-s @F:shortcut.ctrl-i @F:shortcut.ctrl-p", async ({
  page,
}) => {
  await setup(page);
  await focusEditor(page);
  // Orca : Ctrl+O ouvre le sélecteur de fichier projet (.3mf).
  const projectChooser = page.waitForEvent("filechooser");
  await page.keyboard.press("Control+o");
  const pc = await projectChooser;
  expect(await pc.element().getAttribute("accept")).toMatch(/3mf/);
  await focusEditor(page);
  const saved = page.waitForResponse(
    (r) =>
      r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url()),
  );
  await page.keyboard.press("Control+s");
  expect((await saved).status()).toBeLessThan(300);
  await expect(
    page.getByText("Projet sauvegardé.", { exact: true }).first(),
  ).toBeVisible();

  const oldUrl = page.url();
  await page.keyboard.press("Control+Shift+S");
  await expect(page).not.toHaveURL(oldUrl, { timeout: 30_000 });
  await expect(page).toHaveURL(/\/projects\/[^/]+$/);
  // La copie reprend la scène enregistrée (le cube).
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });

  const chooser = page.waitForEvent("filechooser");
  await focusEditor(page);
  await page.keyboard.press("Control+i");
  await (await chooser).setFiles(fixture("cylinder"));
  await expect(page.getByTestId("object-row")).toHaveCount(2, {
    timeout: 60_000,
  });
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  await expect(page.getByRole("dialog")).toBeVisible();
});

test("presse-papier, sélection, suppression et historique @F:shortcut.ctrl-x @F:shortcut.ctrl-c @F:shortcut.ctrl-v @F:shortcut.ctrl-d @F:shortcut.fn @F:shortcut.esc-2 @F:shortcut.1-9-2 @F:shortcut.del-2 @F:shortcut.esc-3 @F:shortcut.ctrl-c-2 @F:shortcut.ctrl-v-2 @F:shortcut.ctrl-x-2 @F:shortcut.ctrl-a-2 @F:shortcut.ctrl-z-2 @F:shortcut.ctrl-y-2 @F:shortcut.mouse-click", async ({
  page,
}) => {
  await setup(page, ["cube20", "cylinder"]);
  const rows = page.getByTestId("object-row");
  const selected = page.locator('[data-testid="object-row"].bg-primary');
  // Historique : un instantané est empilé 400 ms après chaque modification.
  const settle = () => page.waitForTimeout(800);
  await settle();
  await focusEditor(page);
  await page.keyboard.press("Control+a");
  await expect(selected).toHaveCount(2);
  await page.keyboard.press("Control+c");
  await page.keyboard.press("Control+v");
  await expect(rows).toHaveCount(4);
  await settle();
  await page.keyboard.press("Control+z");
  await expect(rows).toHaveCount(2);
  await settle();
  await page.keyboard.press("Control+y");
  await expect(rows).toHaveCount(4);
  await page.keyboard.press("Escape");
  await expect(selected).toHaveCount(0);

  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("Control+x");
  await expect(rows).toHaveCount(3);
  await page.keyboard.press("Control+v");
  await expect(rows).toHaveCount(4);
  await settle();
  await page.keyboard.press("Control+a");
  await page.keyboard.press("Backspace");
  await expect(rows).toHaveCount(0);
  await settle();
  await page.keyboard.press("Control+z");
  await expect(rows).toHaveCount(4);
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("Delete");
  await expect(rows).toHaveCount(3);
  await settle();
  await page.keyboard.press("Control+d");
  await expect(rows).toHaveCount(0);
  await settle();
  await page.keyboard.press("Control+z");
  await expect(rows).toHaveCount(3);

  await page.getByTestId("object-name").first().dblclick();
  const rename = page.getByTestId("object-rename-input");
  await expect(rename).toBeFocused();
  await rename.fill("Cube renommé");
  await rename.press("Enter");
  await expect(page.getByTestId("object-name").first()).toHaveText(
    /Cube renommé/,
  );

  await page.getByRole("button", { name: "Traitement", exact: true }).click();
  await page.getByTestId("add-filament-slot").click();
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("2");
  await expect(page.getByLabel(/^Extrudeur de /).first()).toHaveValue("2");
});

test("gizmos, imprimable, zoom et panneau latéral @F:shortcut.h @F:shortcut.p @F:shortcut.t @F:shortcut.u @F:shortcut.y @F:shortcut.e @F:shortcut.i @F:shortcut.o @F:shortcut.v @F:shortcut.shift-tab", async ({
  page,
}) => {
  await setup(page);
  await page.getByTestId("object-name").click();
  await focusEditor(page);
  for (const [key, tool] of [
    ["h", "fuzzy-paint"],
    ["p", "seam-paint"],
    ["t", "emboss"],
    ["u", "measure"],
    ["y", "assembly"],
    ["e", "brim-ears"],
  ] as const) {
    await page.keyboard.press(key);
    await expect(page.getByTestId(`tool-${tool}`)).toHaveAttribute(
      "aria-pressed",
      "true",
      { timeout: 20_000 },
    );
  }
  const canvas = page.getByTestId("scene-canvas");
  const zoom = Number(await canvas.getAttribute("data-camera-zoom"));
  await page.keyboard.press("i");
  await expect
    .poll(async () => Number(await canvas.getAttribute("data-camera-zoom")))
    .toBeGreaterThan(zoom);
  await page.keyboard.press("o");
  await expect(canvas).toHaveAttribute("data-camera-zoom", zoom.toFixed(3));
  await page.keyboard.press("v");
  await expect(page.getByTestId("object-row")).toHaveAttribute(
    "data-printable",
    "false",
  );
  await page.keyboard.press("Shift+Tab");
  await expect(page.locator("aside").first()).toBeHidden();
  await page.keyboard.press("Shift+Tab");
  await expect(page.locator("aside").first()).toBeVisible();
});

test("caméras nommées et déplacement dans le repère caméra @F:shortcut.ctrl-0 @F:shortcut.ctrl-2 @F:shortcut.ctrl-3 @F:shortcut.ctrl-4 @F:shortcut.ctrl-5 @F:shortcut.ctrl-6 @F:shortcut.ctrl-any-arrow", async ({
  page,
}) => {
  await setup(page);
  const canvas = page.getByTestId("scene-canvas");
  await focusEditor(page);
  for (const [key, view] of [
    ["2", "bottom"],
    ["3", "front"],
    ["4", "rear"],
    ["5", "left"],
    ["6", "right"],
    ["0", "default"],
  ] as const) {
    await page.keyboard.press(`Control+${key}`);
    await expect(canvas).toHaveAttribute("data-camera-view", view);
  }
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  const before = (await positions(page))[0].xyz;
  await page.keyboard.press("Control+ArrowUp");
  await expect
    .poll(async () => (await positions(page))[0].xyz[0])
    .not.toBe(before[0]);
  const after = (await positions(page))[0].xyz;
  expect(after[0]).not.toBe(before[0]);
  expect(after[1]).not.toBe(before[1]);
});

test("molette de zoom de la scène @F:shortcut.mouse-wheel", async ({
  page,
}) => {
  await setup(page);
  const canvas = page.getByTestId("scene-canvas");
  const before = await canvas.screenshot();
  await canvas.hover({ position: { x: 400, y: 300 } });
  await page.mouse.wheel(0, -500);
  await page.waitForTimeout(800);
  const after = await canvas.screenshot();
  expect(after.equals(before)).toBe(false);
});

test("arrangement et orientation au clavier @F:shortcut.a @F:shortcut.shift-a @F:shortcut.q @F:shortcut.shift-q", async ({
  page,
}) => {
  await setup(page, ["overhang", "cylinder"]);
  await focusEditor(page);
  const arranged = page.waitForResponse(
    (r) => /\/arrange$/.test(r.url()) && r.request().method() === "POST",
  );
  await page.keyboard.press("a");
  const arrangeRes = await arranged;
  expect(arrangeRes.status(), await arrangeRes.text()).toBeLessThan(300);
  await expect(page.getByText(/Arrangement : 2 objet/)).toBeVisible();
  const first = JSON.stringify(await positions(page));
  // Décale un objet puis Maj+A : l'arrangement du plateau actif le replace.
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("ArrowUp");
  await expect
    .poll(async () => JSON.stringify(await positions(page)))
    .not.toBe(first);
  const nudged = JSON.stringify(await positions(page));
  await page.keyboard.press("Escape");
  const rearranged = page.waitForResponse(
    (r) => /\/arrange$/.test(r.url()) && r.request().method() === "POST",
  );
  await page.keyboard.press("Shift+A");
  expect((await rearranged).status()).toBeLessThan(300);
  await expect
    .poll(async () => JSON.stringify(await positions(page)))
    .not.toBe(nudged);

  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  const oriented = page.waitForResponse(
    (r) => /\/orient$/.test(r.url()) && r.request().method() === "POST",
  );
  await page.keyboard.press("q");
  expect((await oriented).status()).toBeLessThan(300);
  await expect(
    page.getByText("Orientation auto appliquée.", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Shift+Q");
  await expect(
    page.getByText("Orientation auto : 2 objet(s) du plateau.", {
      exact: true,
    }),
  ).toBeVisible();
});

test("peinture : Ctrl+molette change le rayon @F:shortcut.ctrl-mouse-wheel", async ({
  page,
}) => {
  await setup(page);
  await page.getByTestId("object-name").click();
  await focusEditor(page);
  await page.keyboard.press("p");
  const radius = page.locator('input[type="range"][min="0.2"][max="20"]');
  await expect(radius).toHaveValue("2");
  await page
    .getByTestId("scene-canvas")
    .hover({ position: { x: 400, y: 300 } });
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -120);
  await page.keyboard.up("Control");
  await expect(radius).toHaveValue("2.2");
});

test("aperçu : curseurs, mode une couche, G-code, accélérations et export/impression @F:shortcut.arrow-up-2 @F:shortcut.arrow-down-2 @F:shortcut.arrow-left-2 @F:shortcut.arrow-right-2 @F:shortcut.l-2 @F:shortcut.c-2 @F:shortcut.tab-2 @F:shortcut.shift-any-arrow-2 @F:shortcut.shift-mouse-wheel @F:shortcut.ctrl-any-arrow-2 @F:shortcut.ctrl-mouse-wheel-2 @F:shortcut.home @F:shortcut.end @F:shortcut.ctrl-g @F:shortcut.ctrl-shift-g", async ({
  page,
}) => {
  await setup(page);
  await sliceInUi(page);
  const canvas = page.getByTestId("preview-canvas");
  await expect(canvas).toBeVisible();
  await focusEditor(page);

  await page.keyboard.press("Home");
  await expect(canvas).toHaveAttribute("data-move-cursor", "0");
  await page.keyboard.press("ArrowRight");
  await expect(canvas).toHaveAttribute("data-move-cursor", "1");
  await page.keyboard.press("Shift+ArrowRight");
  await expect(canvas).toHaveAttribute("data-move-cursor", "6");
  await page.keyboard.press("Control+ArrowRight");
  await expect(canvas).toHaveAttribute("data-move-cursor", "11");
  await page.keyboard.press("ArrowLeft");
  await expect(canvas).toHaveAttribute("data-move-cursor", "10");
  await page.keyboard.press("End");
  const end = Number(await canvas.getAttribute("data-move-cursor"));
  expect(end).toBeGreaterThan(10);

  const initialHigh = Number(await canvas.getAttribute("data-layer-high"));
  await page.keyboard.press("ArrowDown");
  await expect(canvas).toHaveAttribute(
    "data-layer-high",
    String(initialHigh - 1),
  );
  await page.keyboard.press("Shift+ArrowDown");
  await expect(canvas).toHaveAttribute(
    "data-layer-high",
    String(initialHigh - 6),
  );
  await page.keyboard.press("Control+ArrowUp");
  await expect(canvas).toHaveAttribute(
    "data-layer-high",
    String(initialHigh - 1),
  );
  await page.keyboard.press("ArrowUp");
  await expect(canvas).toHaveAttribute("data-layer-high", String(initialHigh));

  await canvas.hover({ position: { x: 500, y: 350 } });
  await page.keyboard.down("Shift");
  await page.mouse.wheel(0, 120);
  await page.keyboard.up("Shift");
  await expect(canvas).toHaveAttribute(
    "data-layer-high",
    String(initialHigh - 5),
  );
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -120);
  await page.keyboard.up("Control");
  await expect(canvas).toHaveAttribute("data-layer-high", String(initialHigh));

  await focusEditor(page);
  await page.keyboard.press("l");
  await expect(canvas).toHaveAttribute("data-layer-low", String(initialHigh));
  await page.keyboard.press("l");
  await expect(canvas).toHaveAttribute("data-layer-low", "0");
  await page.keyboard.press("c");
  await expect(page.getByTestId("gcode-window")).toBeVisible();
  await page.keyboard.press("c");
  await expect(page.getByTestId("gcode-window")).toHaveCount(0);

  const download = page.waitForEvent("download");
  await page.keyboard.press("Control+g");
  expect((await download).suggestedFilename()).toMatch(/\.gcode$/);
  await page.keyboard.press("Control+Shift+G");
  await expect(page.getByTestId("send-dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await focusEditor(page);
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("preview-view")).toBeVisible();
});

// — Souris dans la scène 3D : ancres écran exposées par la scène
//   (data-object-anchors = "id:x,y;objet/pièce:x,y", px CSS du canvas). —
async function anchorsOf(
  page: Page,
): Promise<Record<string, [number, number]>> {
  const text =
    (await page
      .getByTestId("scene-canvas")
      .getAttribute("data-object-anchors")) ?? "";
  return Object.fromEntries(
    text
      .split(";")
      .filter(Boolean)
      .map((entry) => {
        const i = entry.lastIndexOf(":");
        const [x, y] = entry
          .slice(i + 1)
          .split(",")
          .map(Number);
        return [entry.slice(0, i), [x, y] as [number, number]];
      }),
  );
}

async function canvasPoint(page: Page, [x, y]: [number, number]) {
  const box = (await page.getByTestId("scene-canvas").boundingBox())!;
  return { x: box.x + x, y: box.y + y };
}

/** Aucun outil actif : pas de gizmo par-dessus les objets visés. */
async function noTool(page: Page) {
  const active = page.locator('[data-testid^="tool-"][aria-pressed="true"]');
  if (await active.count()) await active.first().click();
  await expect(active).toHaveCount(0);
}

const selectedRows = (page: Page) =>
  page.locator('[data-testid="object-row"].bg-primary');

async function objectAnchors(page: Page, count: number) {
  await expect
    .poll(
      async () =>
        Object.keys(await anchorsOf(page)).filter((k) => !k.includes("/"))
          .length,
    )
    .toBe(count);
  return Object.entries(await anchorsOf(page)).filter(
    ([k]) => !k.includes("/"),
  );
}

test("Ctrl+clic et Maj+rectangle : sélection multiple dans la scène @F:shortcut.ctrl-left-mouse-button @F:shortcut.shift-left-mouse-button", async ({
  page,
}) => {
  await setup(page, ["cube20", "cylinder"]);
  await noTool(page);
  // Écarte le cylindre (5 × 10 mm) pour des projections disjointes.
  await page.getByTestId("object-name").nth(1).click();
  await focusEditor(page);
  for (let i = 0; i < 5; i++) await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Escape");
  await expect(selectedRows(page)).toHaveCount(0);
  await page.waitForTimeout(500);
  const [[, a], [, b]] = (await objectAnchors(page, 2)).sort(
    (l, r) => l[1][0] - r[1][0],
  );

  // Clic simple puis Ctrl+clic : ajout ; Ctrl+clic à nouveau : retrait.
  const pa = await canvasPoint(page, a);
  const pb = await canvasPoint(page, b);
  await page.mouse.click(pa.x, pa.y);
  await expect(selectedRows(page)).toHaveCount(1);
  await page.keyboard.down("Control");
  await page.mouse.click(pb.x, pb.y);
  await page.keyboard.up("Control");
  await expect(selectedRows(page)).toHaveCount(2);
  await page.keyboard.down("Control");
  await page.mouse.click(pa.x, pa.y);
  await page.keyboard.up("Control");
  await expect(selectedRows(page)).toHaveCount(1);

  // Maj+glisser depuis le vide sur les deux objets : rectangle de sélection.
  await page.keyboard.press("Escape");
  await expect(selectedRows(page)).toHaveCount(0);
  const box = (await page.getByTestId("scene-canvas").boundingBox())!;
  const start = { x: box.x + 8, y: box.y + box.height - 8 };
  const end = await canvasPoint(page, [
    Math.max(a[0], b[0]) + 40,
    Math.min(a[1], b[1]) - 40,
  ]);
  await page.keyboard.down("Shift");
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps: 8 });
  await expect(page.getByTestId("selection-rectangle")).toBeVisible();
  await page.mouse.up();
  await page.keyboard.up("Shift");
  await expect(page.getByTestId("selection-rectangle")).toHaveCount(0);
  await expect(selectedRows(page)).toHaveCount(2);
});

test("Alt+clic : sélection de la pièce sous le curseur @F:shortcut.alt-left-mouse-button", async ({
  page,
}) => {
  await setup(page);
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page
    .getByRole("menuitem", { name: /^Add part|Ajouter une pièce/i })
    .click();
  await noTool(page);
  await focusEditor(page);
  await page.keyboard.press("Escape");
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).not.toHaveAttribute("data-selected-volume", /.+/);
  await expect
    .poll(async () =>
      Object.keys(await anchorsOf(page)).some((k) => k.includes("/")),
    )
    .toBe(true);
  const anchors = await anchorsOf(page);
  const partKey = Object.keys(anchors).find((k) => k.includes("/"))!;
  const partId = partKey.split("/")[1];
  const point = await canvasPoint(page, anchors[partKey]);

  // Clic simple : l'objet entier, sans pièce.
  await page.mouse.click(point.x, point.y);
  await expect(selectedRows(page)).toHaveCount(1);
  await expect(canvas).not.toHaveAttribute("data-selected-volume", /.+/);
  // Alt+clic : la pièce.
  await page.keyboard.down("Alt");
  await page.mouse.click(point.x, point.y);
  await page.keyboard.up("Alt");
  await expect(canvas).toHaveAttribute("data-selected-volume", partId);
  // Clic simple sur l'objet : objet entier, la pièce est désélectionnée (Orca).
  await page.mouse.click(point.x, point.y);
  await expect(canvas).not.toHaveAttribute("data-selected-volume", /.+/);
  await expect(selectedRows(page)).toHaveCount(1);
});

test("Maj pendant le glisser du gizmo Move : pas de 1 mm @F:shortcut.shift", async ({
  page,
}) => {
  await setup(page);
  await page.getByTestId("object-name").first().click();
  const move = page.getByTestId("tool-move");
  if ((await move.getAttribute("aria-pressed")) !== "true") await move.click();
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
  const before = (await positions(page))[0].xyz;
  const [ax, ay] = (await canvas.getAttribute("data-object-anchor"))!
    .split(",")
    .map(Number);
  const p = await canvasPoint(page, [ax, ay]);

  // Poignée centrale (déplacement dans le plan de vue), Maj maintenue.
  await page.mouse.move(p.x, p.y);
  await page.keyboard.down("Shift");
  await page.mouse.down();
  for (let i = 1; i <= 4; i++) {
    await page.mouse.move(p.x + i * 17, p.y + i * 7, { steps: 4 });
    await page.waitForTimeout(150);
  }
  await page.mouse.up();
  await page.keyboard.up("Shift");
  await page.waitForTimeout(600);

  const after = (await positions(page))[0].xyz;
  expect(
    Math.hypot(after[0] - before[0], after[1] - before[1]),
  ).toBeGreaterThan(1);
  for (const v of after.slice(0, 2))
    expect(Math.abs(v - Math.round(v))).toBeLessThan(1e-6);
});

test("Alt+molette en peinture : plan de coupe orienté caméra ; Ctrl+molette garde le rayon @F:shortcut.alt-mouse-wheel", async ({
  page,
}) => {
  await setup(page);
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("p");
  const canvas = page.getByTestId("scene-canvas");
  const radius = page.locator('input[type="range"][min="0.2"][max="20"]');
  await expect(radius).toHaveValue("2");
  await expect(canvas).toHaveAttribute("data-section-ratio", "0.00");
  await expect(canvas).not.toHaveAttribute("data-section-normal", /.+/);

  const numbers = async (attr: string) =>
    ((await canvas.getAttribute(attr)) ?? "").split(",").map(Number);
  const center = (await positions(page))[0].xyz;
  // Cube de 20 mm : rayon de la sphère englobante = √1200 / 2.
  const R = Math.sqrt(1200) / 2;
  const expectPlane = async (n: number[], ratio: number) => {
    const got = await numbers("data-section-normal");
    for (let i = 0; i < 3; i++) expect(got[i]).toBeCloseTo(n[i], 2);
    const dist = n[0] * center[0] + n[1] * center[1] + n[2] * center[2];
    const [offset] = await numbers("data-section-offset");
    expect(offset).toBeCloseTo(dist + R - ratio * 2 * R, 1);
  };

  await canvas.hover({ position: { x: 400, y: 300 } });
  await page.keyboard.down("Alt");
  for (let i = 0; i < 3; i++) await page.mouse.wheel(0, -120);
  await page.mouse.wheel(0, 120);
  await page.keyboard.up("Alt");
  await expect(canvas).toHaveAttribute("data-section-ratio", "0.02");
  // Vue par défaut : normale = direction caméra → cible, inversée (vers la caméra).
  const l = Math.hypot(-0.6, -0.8, 0.7);
  await expectPlane([-0.6 / l, -0.8 / l, 0.7 / l], 0.02);
  await expect(radius).toHaveValue("2");

  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -120);
  await page.keyboard.up("Control");
  await expect(radius).toHaveValue("2.2");
  await expect(canvas).toHaveAttribute("data-section-ratio", "0.02");

  // Fermer le gizmo réinitialise la coupe ; en vue de dessus, normale +Z.
  await focusEditor(page);
  await page.keyboard.press("p");
  await expect(canvas).toHaveAttribute("data-section-ratio", "0.00");
  await page.keyboard.press("Control+1");
  await expect(canvas).toHaveAttribute("data-camera-view", "top");
  await page.waitForTimeout(500);
  await page.keyboard.press("p");
  await canvas.hover({ position: { x: 400, y: 300 } });
  await page.keyboard.down("Alt");
  await page.mouse.wheel(0, -120);
  await page.keyboard.up("Alt");
  await expect(canvas).toHaveAttribute("data-section-ratio", "0.01");
  await expectPlane([0, 0, 1], 0.01);
});
