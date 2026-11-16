// Lot ctx-menus (2) : menu d'une pièce (`create_part_menu` / `create_bbl_part_menu`),
// split en multi-sélection (`multi_selection_menu`) et subdivision de Loop
// (`ObjectList::smooth_mesh`). Effets vérifiés dans la liste d'objets.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

const rows = (page: Page) => page.getByTestId("object-row");
const names = (page: Page) => page.getByTestId("object-name");
/** Lignes d'objet multi-pièces (nœud groupe, glyphe ▾). */
const groups = (page: Page) => names(page).filter({ hasText: "▾" });
const msg = (page: Page) => page.getByTestId("editor-message");

async function setup(page: Page, models: string[]) {
  await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(rows(page)).toHaveCount(models.length, { timeout: 60_000 });
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

async function menuItem(page: Page, name: RegExp) {
  const item = page.getByRole("menuitem", { name });
  await item.scrollIntoViewIfNeeded();
  await item.click();
}

async function rowMenu(
  page: Page,
  row: ReturnType<Page["locator"]>,
  name: RegExp,
) {
  await row.click({ button: "right" });
  await menuItem(page, name);
}

const rowNamed = (page: Page, text: string | RegExp) =>
  rows(page).filter({ has: names(page).filter({ hasText: text }) });

async function selectAll(page: Page, n: number) {
  await names(page).first().click();
  for (let i = 1; i < n; i++)
    await names(page)
      .nth(i)
      .click({ modifiers: ["Control"] });
  await expect(
    page.locator('[data-testid="object-row"].bg-primary'),
  ).toHaveCount(n);
}

/**
 * Pièces de l'objet multi-pièces, dans l'ordre de la liste. Orca
 * `ModelVolume::split` : la pièce est remplacée à sa place par `<nom>_1`,
 * `<nom>_2`… (nom = nom du volume, fichier STL compris), frères des autres
 * pièces du même objet.
 */
async function expectSplitParts(page: Page) {
  await expect(names(page)).toHaveText([
    /Assemblage/,
    /twoparts\.stl_1/,
    /twoparts\.stl_2/,
    /cube20\.stl/,
  ]);
  await expect(groups(page)).toHaveCount(1);
  // Morceaux = pièces maillées propres (pas de volume rattaché à une pièce).
  await names(page).filter({ hasText: "twoparts.stl_2" }).click();
  await expect(page.getByTestId("object-volume")).toHaveCount(0);
}

/** Objet multi-pièces « Assemblage » : twoparts (2 composantes) + cube20. */
async function assembly(page: Page) {
  await setup(page, ["twoparts", "cube20"]);
  await selectAll(page, 2);
  await rowMenu(page, rows(page).first(), /^Assemble$|^Assembler$/);
  await expect(rows(page)).toHaveCount(3);
  await expect(groups(page)).toHaveCount(1);
}

test.describe("menu d’une pièce", () => {
  test("Split sépare la pièce en pièces du même objet @F:ctx.split-2 @F:ctx.split-3", async ({
    page,
  }) => {
    await assembly(page);
    await rowMenu(page, rowNamed(page, "twoparts"), /^Split$|^Séparer$/);
    // Orca `create_part_menu` › Split = `Plater::split_volume` → `ObjectList::split`
    // → `ModelVolume::split` : groupe + twoparts.stl_1 + twoparts.stl_2 + cube20,
    // toujours un seul objet multi-pièces.
    await expect(msg(page)).toContainText("Séparation en pièces : 2 pièce(s)", {
      timeout: 90_000,
    });
    await expect(rows(page)).toHaveCount(4);
    await expectSplitParts(page);
  });

  test("To parts (sous-menu Split) garde les morceaux dans l’objet @F:ctx.to-parts-3", async ({
    page,
  }) => {
    await assembly(page);
    await rowMenu(page, rowNamed(page, "twoparts"), /^To parts$|^En pièces$/);
    // Même action que Split (`split_volume`) : morceaux frères dans le même objet.
    await expect(msg(page)).toContainText("Séparation en pièces : 2 pièce(s)", {
      timeout: 90_000,
    });
    await expect(rows(page)).toHaveCount(4);
    await expectSplitParts(page);
  });

  test("To objects sépare tout l’objet parent en objets (split_object) @F:ctx.to-objects-3", async ({
    page,
  }) => {
    await assembly(page);
    // Depuis la pièce cube20 : Orca `split_object` agit sur l'objet entier —
    // twoparts (2 composantes) + cube20 → 3 objets indépendants.
    await rowMenu(page, rowNamed(page, "cube20"), /^To objects$|^En objets$/);
    await expect(msg(page)).toContainText("Séparation en objets : 3 objet(s)", {
      timeout: 90_000,
    });
    await expect(groups(page)).toHaveCount(0);
    await expect(rows(page)).toHaveCount(3);
    // Une seule étape d'historique (« Split to Objects ») : Ctrl+Z rend l'assemblage.
    await page.waitForTimeout(1200);
    await page.locator("body").focus();
    await page.keyboard.press("Control+z");
    await expect(groups(page)).toHaveCount(1, { timeout: 30_000 });
    await expect(rows(page)).toHaveCount(3);
  });
});

test.describe("multi-sélection", () => {
  test("To objects sépare chaque objet sélectionné, en une étape d’historique @F:ctx.to-objects-4", async ({
    page,
  }) => {
    await setup(page, ["twoparts", "twoparts"]);
    await selectAll(page, 2);
    await rowMenu(page, rows(page).first(), /^To objects$|^En objets$/);
    await expect(rows(page)).toHaveCount(4, { timeout: 120_000 });
    await expect(msg(page)).toContainText(
      "2 objet(s) séparé(s) en 4 morceau(x)",
    );
    await page.waitForTimeout(1200);
    await page.locator("body").focus();
    await page.keyboard.press("Control+z");
    await expect(rows(page)).toHaveCount(2, { timeout: 30_000 });
  });

  test("To parts sépare chaque objet sélectionné en pièces @F:ctx.to-parts-4", async ({
    page,
  }) => {
    await setup(page, ["twoparts", "twoparts"]);
    await selectAll(page, 2);
    await rowMenu(page, rows(page).first(), /^To parts$|^En pièces$/);
    // Deux objets, chacun avec un volume pièce maillé, en une étape d'historique.
    await expect(msg(page)).toContainText("2 objet(s) séparé(s) en pièces", {
      timeout: 120_000,
    });
    await expect(rows(page)).toHaveCount(2);
    await expect(groups(page)).toHaveCount(0);
    // Objet : Orca `ModelVolume::split` garde le nom de l'objet ; ses volumes
    // sont listés : `_1` (maillage de l'objet) puis `_2`.
    for (let i = 0; i < 2; i++) {
      await names(page).nth(i).click();
      await expect(names(page).nth(i)).toContainText("twoparts.stl");
      await expect(page.getByTestId("object-volume")).toHaveText([
        /twoparts\.stl_1/,
        /twoparts\.stl_2/,
      ]);
    }
    await page.waitForTimeout(1200);
    await page.locator("body").focus();
    await page.keyboard.press("Control+z");
    await names(page).first().click();
    await expect(page.getByTestId("object-volume")).toHaveCount(0, {
      timeout: 30_000,
    });
    await expect(rows(page)).toHaveCount(2);
  });
});

test("Subdivision mesh (Lost color) : ×4 triangles côté moteur @F:ctx.subdivision-mesh-lost-color", async ({
  page,
}) => {
  await setup(page, ["cube20"]);
  await rowMenu(
    page,
    rows(page).first(),
    /^Subdivision mesh|^Subdiviser le maillage/,
  );
  // cube20.stl : 12 facettes → 48 après une subdivision de Loop.
  await expect(msg(page)).toContainText("Subdivision : 48 triangles", {
    timeout: 90_000,
  });
  await expect(rows(page)).toHaveCount(1);
});
