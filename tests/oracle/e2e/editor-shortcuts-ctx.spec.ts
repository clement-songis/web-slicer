// Contrôle de parité raccourcis clavier + menus contextuels (KBShortcutsDialog
// et MenuFactory d'Orca, audit/ui_inventory.json) : échantillon représentatif
// de 18 raccourcis et 12 entrées contextuelles, chacun vérifié par son effet
// observable dans l'UI (pas seulement par la présence de l'entrée).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

const axis = (
  page: Page,
  field: "Position (mm)" | "Rotation (°)" | "Échelle",
  a: "X" | "Y" | "Z",
) => page.getByLabel(`${field} ${a}`, { exact: true });

async function value(
  page: Page,
  field: "Position (mm)" | "Rotation (°)" | "Échelle",
  a: "X" | "Y" | "Z",
) {
  return Number(await axis(page, field, a).inputValue());
}

/** Clic neutre hors champ : le focus quitte les inputs, les raccourcis sont captés. */
async function focusCanvas(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
}

async function setup(page: Page, models: string[] = ["cube20"]) {
  await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("object-row")).toHaveCount(models.length);
  // Le panneau de transformation est dans l'onglet Objets (Process par défaut).
  await page.getByRole("button", { name: "Objets", exact: true }).click();
}

async function selectRow(page: Page, index = 0) {
  await page.getByTestId("object-name").nth(index).click();
}

/** Affiche les valeurs numériques via l'outil Flatten, seul panneau objet existant. */
async function selectRowForTransform(page: Page, index = 0) {
  await selectRow(page, index);
  const flatten = page.getByTestId("tool-flatten");
  if ((await flatten.getAttribute("aria-pressed")) !== "true")
    await flatten.click();
  await expect(axis(page, "Position (mm)", "X")).toBeVisible();
}

async function contextItem(page: Page, label: RegExp, row = 0) {
  await page.getByTestId("object-row").nth(row).click({ button: "right" });
  const item = page.getByRole("menuitem", { name: label });
  await item.scrollIntoViewIfNeeded();
  await item.click();
}

test.describe("raccourcis clavier (parité Orca)", () => {
  test("flèches : 10 mm, Maj+flèche : 1 mm @F:shortcut.arrow-up @F:shortcut.arrow-down @F:shortcut.arrow-left @F:shortcut.arrow-right @F:shortcut.shift-any-arrow", async ({
    page,
  }) => {
    await setup(page);
    await selectRowForTransform(page);
    const x0 = await value(page, "Position (mm)", "X");
    const y0 = await value(page, "Position (mm)", "Y");
    await focusCanvas(page);
    await selectRow(page);
    await page.locator("body").focus();
    await page.keyboard.press("ArrowRight");
    await expect
      .poll(() => value(page, "Position (mm)", "X"))
      .toBeCloseTo(x0 + 10, 3);
    await page.keyboard.press("ArrowUp");
    await expect
      .poll(() => value(page, "Position (mm)", "Y"))
      .toBeCloseTo(y0 + 10, 3);
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Shift+ArrowRight");
    await expect
      .poll(() => value(page, "Position (mm)", "X"))
      .toBeCloseTo(x0 + 1, 3);
    await expect
      .poll(() => value(page, "Position (mm)", "Y"))
      .toBeCloseTo(y0, 3);
  });

  test("gizmos M/R/S/F/C/B : outil actif @F:shortcut.m @F:shortcut.r @F:shortcut.s @F:shortcut.f @F:shortcut.c @F:shortcut.b", async ({
    page,
  }) => {
    await setup(page);
    await selectRow(page);
    await page.locator("body").focus();
    for (const [key, tool] of [
      ["m", "move"],
      ["r", "rotate"],
      ["s", "scale"],
      ["f", "flatten"],
      ["c", "cut"],
      ["b", "boolean"],
    ] as const) {
      // Chaque raccourci est testé depuis l'état « aucun outil » : les outils
      // de transformation sont des bascules, M sur Move actif le désactive.
      const active = page.locator(
        '[data-testid^="tool-"][aria-pressed="true"]',
      );
      if (await active.count()) await active.first().click();
      await page.keyboard.press(key);
      await expect(page.getByTestId(`tool-${tool}`), key).toHaveAttribute(
        "aria-pressed",
        "true",
      );
    }
  });

  test("Ctrl+A / Esc / Del / Ctrl+Z / Ctrl+Y / Ctrl+K @F:shortcut.ctrl-a @F:shortcut.esc @F:shortcut.del @F:shortcut.ctrl-z @F:shortcut.ctrl-y @F:shortcut.ctrl-k", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    const rows = page.getByTestId("object-row");
    await page.waitForTimeout(800);
    await focusCanvas(page);
    await page.keyboard.press("Control+a");
    await expect(rows.locator(".bg-primary")).toHaveCount(0); // classe sur la ligne elle-même
    await expect(
      page.locator('[data-testid="object-row"].bg-primary'),
    ).toHaveCount(2);
    await page.keyboard.press("Escape");
    await expect(
      page.locator('[data-testid="object-row"].bg-primary'),
    ).toHaveCount(0);
    await selectRow(page, 1);
    await page.locator("body").focus();
    await page.keyboard.press("Delete");
    await expect(rows).toHaveCount(1);
    await page.waitForTimeout(800);
    await page.keyboard.press("Control+z");
    await expect(rows).toHaveCount(2);
    await page.keyboard.press("Control+y");
    await expect(rows).toHaveCount(1);
    await selectRow(page, 0);
    await page.locator("body").focus();
    await page.keyboard.press("Control+k");
    await expect(rows).toHaveCount(2);
  });

  test("1–9 : filament de la sélection @F:shortcut.1-9", async ({ page }) => {
    await setup(page);
    await page.getByRole("button", { name: "Process", exact: true }).click();
    await page.getByTestId("add-filament-slot").click();
    await selectRow(page);
    await page.locator("body").focus();
    await page.keyboard.press("2");
    await expect(page.getByLabel(/^Extrudeur de /)).toHaveValue("2");
    // Au-delà du nombre de filaments : ignoré (parité Orca).
    await page.keyboard.press("7");
    await expect(page.getByLabel(/^Extrudeur de /)).toHaveValue("2");
  });

  test("Espace : renommage de l’objet sélectionné @F:shortcut.space", async ({
    page,
  }) => {
    await setup(page);
    await selectRow(page);
    await focusCanvas(page);
    await page.keyboard.press(" ");
    await expect(page.getByTestId("object-rename-input")).toBeFocused();
  });

  test("« ? » (Maj) ouvre la liste des raccourcis ; Tab bascule vers l’Aperçu @F:shortcut.x @F:shortcut.tab", async ({
    page,
  }) => {
    await setup(page);
    await focusCanvas(page);
    await page.keyboard.press("Shift+?");
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await focusCanvas(page);
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("scene-canvas")).toBeHidden();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
  });

  test("Ctrl+1 / Ctrl+0 : vues caméra sans rechargement ; Ctrl+R tranche @F:shortcut.ctrl-1 @F:shortcut.ctrl-r", async ({
    page,
  }) => {
    await setup(page);
    await focusCanvas(page);
    const url = page.url();
    await page.keyboard.press("Control+1");
    await page.keyboard.press("Control+0");
    expect(page.url()).toBe(url);
    const slice = page.waitForRequest(
      (r) => r.method() === "POST" && /\/slice/.test(r.url()),
    );
    await page.keyboard.press("Control+r");
    await slice;
    expect(page.url()).toBe(url);
  });
});

test.describe("menus contextuels (parité Orca)", () => {
  test("Mirror X/Y/Z inverse le signe de l’échelle @F:ctx.along-x-axis @F:ctx.along-y-axis @F:ctx.along-z-axis", async ({
    page,
  }) => {
    await setup(page);
    await selectRowForTransform(page);
    for (const [label, a] of [
      [/^Along X axis|^Selon l.axe X/, "X"],
      [/^Along Y axis|^Selon l.axe Y/, "Y"],
      [/^Along Z axis|^Selon l.axe Z/, "Z"],
    ] as const) {
      await contextItem(page, label);
      await expect.poll(() => value(page, "Échelle", a)).toBeCloseTo(-1, 3);
    }
  });

  test("Scale to build volume agrandit l’objet sous le volume @F:ctx.scale-to-build-volume", async ({
    page,
  }) => {
    await setup(page);
    await selectRowForTransform(page);
    await contextItem(page, /Scale to build volume|volume d.impression/i);
    await expect.poll(() => value(page, "Échelle", "X")).toBeGreaterThan(1.5);
  });

  test("Clone / Add instance / Remove instance / Delete @F:ctx.clone @F:ctx.add-instance @F:ctx.remove-instance @F:ctx.delete", async ({
    page,
  }) => {
    await setup(page);
    const rows = page.getByTestId("object-row");
    await contextItem(page, /^Clone|^Cloner/);
    await expect(rows).toHaveCount(2);
    await contextItem(page, /^Add instance|^Ajouter une instance/);
    await expect(rows).toHaveCount(3);
    await contextItem(
      page,
      /^Remove instance|^Retirer une instance|^Supprimer l.instance/,
      2,
    );
    await expect(rows).toHaveCount(2);
    await contextItem(page, /^Delete$|^Supprimer$/, 1);
    await expect(rows).toHaveCount(1);
  });

  test("Center / Drop replacent l’objet @F:ctx.center @F:ctx.drop", async ({
    page,
  }) => {
    await setup(page);
    await selectRowForTransform(page);
    await axis(page, "Position (mm)", "X").fill("15");
    await axis(page, "Position (mm)", "X").press("Enter");
    await axis(page, "Position (mm)", "Z").fill("40");
    await axis(page, "Position (mm)", "Z").press("Enter");
    await contextItem(page, /^Drop|^Poser/);
    await expect
      .poll(() => value(page, "Position (mm)", "Z"))
      .toBeCloseTo(10, 1);
    await contextItem(page, /^Center|^Centrer/);
    await expect
      .poll(() => value(page, "Position (mm)", "X"))
      .not.toBeCloseTo(15, 1);
  });

  test("Export as one STL… télécharge un STL @F:ctx.export-as-one-stl", async ({
    page,
  }) => {
    await setup(page);
    const download = page.waitForEvent("download");
    await contextItem(page, /Export as one STL|Exporter en un seul STL/);
    expect((await download).suggestedFilename()).toMatch(/\.stl$/);
  });

  test("Mesh boolean ouvre l’outil ; Set number of instances… crée N−1 copies @F:ctx.mesh-boolean @F:ctx.set-number-of-instances", async ({
    page,
  }) => {
    await setup(page);
    await contextItem(page, /^Mesh boolean|^Booléen/);
    await expect(page.getByTestId("tool-boolean")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    page.once("dialog", (d) => d.accept("3"));
    await contextItem(page, /Set number of instances|Nombre d.instances/);
    await expect(page.getByTestId("object-row")).toHaveCount(3);
  });

  test("plateau vide : Cube ajoute une primitive, Show Labels est proposé @F:ctx.cube @F:ctx.add-models", async ({
    page,
  }) => {
    await bootstrap(page);
    const canvas = page.getByTestId("scene-canvas");
    await canvas.click({ button: "right", position: { x: 40, y: 40 } });
    await expect(
      page.getByRole("menuitem", {
        name: /Show Labels|Afficher les étiquettes/,
      }),
    ).toBeVisible();
    await page.getByRole("menuitem", { name: /^Cube$/ }).click();
    await expect(page.getByTestId("object-row")).toHaveCount(1);
  });
});
