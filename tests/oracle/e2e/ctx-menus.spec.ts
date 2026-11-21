// Lot ctx-menus : parité des menus contextuels Orca (MenuFactory, GUI_Factories.cpp).
// Chaque test vérifie un effet observable (scène, liste d'objets, plateaux,
// fichier téléchargé), jamais la seule présence d'une entrée de menu.
import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { bootstrap, fixture, importModel } from "../lib/app";

const rows = (page: Page) => page.getByTestId("object-row");

async function ready(page: Page, count: number) {
  await expect(rows(page)).toHaveCount(count, { timeout: 60_000 });
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
}

async function setup(page: Page, models: string[] = ["cube20"]) {
  await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await ready(page, models.length);
}

async function menuItem(page: Page, name: RegExp) {
  const item = page.getByRole("menuitem", { name });
  await item.scrollIntoViewIfNeeded();
  await item.click();
}

async function objectMenu(page: Page, name: RegExp, row = 0) {
  await rows(page).nth(row).click({ button: "right" });
  await menuItem(page, name);
}

async function sceneMenu(page: Page, name: RegExp) {
  // Coin bas-droit : les panneaux flottants occupent le haut et la gauche.
  const canvas = page.getByTestId("scene-canvas");
  const box = (await canvas.boundingBox())!;
  await canvas.click({
    button: "right",
    position: { x: box.width - 80, y: box.height - 80 },
  });
  await menuItem(page, name);
}

async function plateMenu(page: Page, name: RegExp, tab = 0) {
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("plate-tab").nth(tab).click({ button: "right" });
  await menuItem(page, name);
}

async function selectAll(page: Page, n: number) {
  await page.getByTestId("object-name").first().click();
  for (let i = 1; i < n; i++)
    await page
      .getByTestId("object-name")
      .nth(i)
      .click({ modifiers: ["Control"] });
  await expect(
    page.locator('[data-testid="object-row"].bg-primary'),
  ).toHaveCount(n);
}

async function stlSize(page: Page): Promise<number> {
  const dl = page.waitForEvent("download");
  await objectMenu(page, /^Export as one STL|^Exporter en un seul STL/);
  const file = await (await dl).path();
  return readFileSync(file).length;
}

const msg = (page: Page) => page.getByTestId("editor-message");

test.describe("menu d’ajout (plateau vide)", () => {
  test("primitives Cylinder/Sphere/Cone/Disc/Torus ajoutées @F:ctx.cylinder @F:ctx.sphere @F:ctx.cone @F:ctx.disc @F:ctx.torus", async ({
    page,
  }) => {
    await bootstrap(page);
    const names: [RegExp, RegExp][] = [
      [/^Cylinder$|^Cylindre$/, /Cylinder/],
      [/^Sphere$|^Sphère$/, /Sphere/],
      [/^Cone$|^Cône$/, /Cone/],
      [/^Disc$|^Disque$/, /Disc/],
      [/^Torus$|^Tore$/, /Torus/],
    ];
    for (const [i, [label, name]] of names.entries()) {
      // Le clic droit sur la scène ouvre le menu objet si un objet est sélectionné.
      await page.keyboard.press("Escape");
      await sceneMenu(page, label);
      await expect(rows(page)).toHaveCount(i + 1);
      await expect(page.getByTestId("object-name").nth(i)).toContainText(name);
    }
  });

  test("Load… et Add Models importent le fichier choisi @F:ctx.load @F:ctx.add-models-2", async ({
    page,
  }) => {
    await bootstrap(page);
    for (const [i, label] of [
      /^Load|^Charger/,
      /^Add Models$|^Ajouter des modèles$/,
    ].entries()) {
      await page.keyboard.press("Escape");
      const chooser = page.waitForEvent("filechooser");
      await sceneMenu(page, label);
      await (await chooser).setFiles(fixture("cube20"));
      // Ligne ajoutée = import lancé (la conversion moteur peut rester en cours).
      await expect(rows(page)).toHaveCount(i + 1, { timeout: 60_000 });
    }
  });
});

test.describe("menu objet", () => {
  test("HideShow masque puis réaffiche l’objet @F:ctx.hideshow", async ({
    page,
  }) => {
    await setup(page);
    const hide = page.getByRole("button", { name: /^Masquer / });
    await objectMenu(page, /^HideShow$|^Masquer\/Afficher$/);
    await expect(hide).toHaveAttribute("aria-pressed", "true");
    await objectMenu(page, /^HideShow$|^Masquer\/Afficher$/);
    await expect(hide).toHaveAttribute("aria-pressed", "false");
  });

  test("Rename renomme l’objet ; Delete le supprime @F:ctx.rename @F:ctx.delete-2", async ({
    page,
  }) => {
    await setup(page);
    await objectMenu(page, /^Rename$|^Renommer$/);
    const input = page.getByTestId("object-rename-input");
    await expect(input).toBeFocused();
    await input.fill("Pièce renommée");
    await input.press("Enter");
    await expect(page.getByTestId("object-name").first()).toContainText(
      "Pièce renommée",
    );
    await objectMenu(page, /^Delete$|^Supprimer$/);
    await expect(rows(page)).toHaveCount(0);
  });

  test("Fill bed with copies / instances remplit le plateau @F:ctx.fill-bed-with-copies @F:ctx.fill-bed-with-instances", async ({
    page,
  }) => {
    await setup(page);
    await objectMenu(
      page,
      /^Fill bed with copies|^Remplir le plateau de copies/,
    );
    await expect.poll(() => rows(page).count()).toBeGreaterThan(4);
    const filled = await rows(page).count();
    await expect(msg(page)).toContainText(`${filled} instances`);
    // Le remplissage d'instances depuis une autre instance complète la grille.
    await objectMenu(
      page,
      /^Fill bed with instances|^Remplir le plateau d.instances/,
    );
    await expect.poll(() => rows(page).count()).toBeGreaterThan(filled);
  });

  test("Set as an individual object détache l’instance de son modèle @F:ctx.set-as-an-individual-object", async ({
    page,
  }) => {
    await setup(page);
    await objectMenu(page, /^Clone$|^Cloner$/);
    await expect(rows(page)).toHaveCount(2);
    await objectMenu(
      page,
      /^Set as an individual object|^Définir comme objet individuel/,
      1,
    );
    await expect(msg(page)).toContainText("objet individuel");
    // Plus d'instance sœur : « Remove instance » refuse de retirer la copie.
    await objectMenu(page, /^Remove instance|^Retirer une instance/, 1);
    await expect(msg(page)).toContainText("seule instance");
    await expect(rows(page)).toHaveCount(2);
  });

  test("Fix model répare le maillage côté serveur @F:ctx.fix-model", async ({
    page,
  }) => {
    await setup(page);
    await objectMenu(page, /^Fix model$|^Réparer le modèle$/);
    await expect(msg(page)).toContainText(/Maillage réparé : .* triangles/, {
      timeout: 60_000,
    });
  });

  test("Simplify Model : maillage simplifié côté serveur @F:ctx.simplify-model", async ({
    page,
  }) => {
    await setup(page, ["cylinder"]);
    await objectMenu(page, /^Simplify Model$|^Simplifier le modèle$/);
    await page.getByTestId("simplify-mode-ratio").check();
    await page.getByTestId("simplify-ratio").fill("50");
    await page.getByTestId("simplify-apply").click();
    await expect(msg(page)).toContainText(
      /Maillage simplifié : \d+ → \d+ triangles/,
      {
        timeout: 60_000,
      },
    );
  });

  test("Export as STLs… télécharge une archive d’un STL par objet @F:ctx.export-as-stls", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await selectAll(page, 2);
    const dl = page.waitForEvent("download");
    await objectMenu(page, /^Export as STLs|^Exporter en plusieurs STL/);
    const download = await dl;
    expect(download.suggestedFilename()).toMatch(/\.zip$/);
    const zip = readFileSync(await download.path()).toString("latin1");
    expect(zip.match(/\.stl/g)?.length ?? 0).toBeGreaterThanOrEqual(2);
  });

  test("Reload from disk relit le maillage serveur @F:ctx.reload-from-disk", async ({
    page,
  }) => {
    await setup(page);
    const mesh = page.waitForResponse(
      (r) => /\/mesh/.test(r.url()) && r.status() === 200,
    );
    await objectMenu(page, /^Reload from disk|^Recharger depuis le disque/);
    await mesh;
    await expect(msg(page)).toContainText("rechargé depuis le disque");
    await expect(rows(page)).toHaveCount(1);
  });

  test("Replace 3D file… remplace le maillage en gardant l’objet @F:ctx.replace-3d-file", async ({
    page,
  }) => {
    await setup(page);
    const before = await stlSize(page);
    const chooser = page.waitForEvent("filechooser");
    await objectMenu(page, /^Replace 3D file|^Remplacer le fichier 3D/);
    await (await chooser).setFiles(fixture("cylinder"));
    await expect(msg(page)).toContainText("1 maillage(s) remplacé(s)", {
      timeout: 60_000,
    });
    await expect(rows(page)).toHaveCount(1);
    expect(await stlSize(page)).not.toBe(before);
  });

  test("Replace all with 3D files… remplace chaque objet sélectionné @F:ctx.replace-all-with-3d-files", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cube20"]);
    await selectAll(page, 2);
    const chooser = page.waitForEvent("filechooser");
    await objectMenu(
      page,
      /^Replace all with 3D files|^Remplacer tout par des fichiers 3D/,
    );
    await (await chooser).setFiles([fixture("cylinder"), fixture("cylinder")]);
    await expect(msg(page)).toContainText("2 maillage(s) remplacé(s)", {
      timeout: 60_000,
    });
    await expect(rows(page)).toHaveCount(2);
  });

  test("Assemble regroupe les objets sélectionnés en un objet multi-pièces @F:ctx.assemble", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await selectAll(page, 2);
    await objectMenu(page, /^Assemble$|^Assembler$/);
    await expect(rows(page)).toHaveCount(3);
    await expect(page.getByTestId("object-name").first()).toContainText(
      "Assemblage",
    );
  });

  test("Assemble (une pièce) fusionne deux objets en un seul @F:ctx.assemble-2", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await selectAll(page, 2);
    await objectMenu(
      page,
      /^Assemble \(single part\)|^Assembler \(une seule pièce\)/,
    );
    await expect(rows(page)).toHaveCount(1, { timeout: 90_000 });
  });

  test("To objects / Split séparent un maillage en objets @F:ctx.to-objects @F:ctx.to-objects-2 @F:ctx.split", async ({
    page,
  }) => {
    await setup(page, ["twoparts"]);
    await objectMenu(page, /^To objects$|^En objets$/);
    await expect(rows(page)).toHaveCount(2, { timeout: 90_000 });
    await expect(msg(page)).toContainText("Séparation : 2 objet(s)");
  });

  test("To parts sépare en pièces d’un même objet @F:ctx.to-parts @F:ctx.to-parts-2", async ({
    page,
  }) => {
    await setup(page, ["twoparts"]);
    await objectMenu(page, /^To parts$|^En pièces$/);
    // Orca split_volume : un seul objet, la 2e composante devient un volume
    // pièce maillé (shape « mesh ») du même objet.
    await expect(page.getByTestId("editor-message")).toContainText(
      "Séparation en pièces : 2 pièce(s)",
      {
        timeout: 90_000,
      },
    );
    await expect(rows(page)).toHaveCount(1);
    await expect(page.getByTestId("object-name").first()).toContainText(
      "twoparts",
    );
    // Lignes de volume Orca : twoparts.stl_1 (maillage de l'objet) et twoparts.stl_2.
    await expect(page.getByTestId("object-volume")).toHaveText([
      /twoparts\.stl_1/,
      /twoparts\.stl_2/,
    ]);
    await expect(
      page.locator('[data-testid="object-volume"][data-type="part"]'),
    ).toHaveCount(2);
  });

  test("Auto orientation applique la rotation calculée par le moteur @F:ctx.auto-orientation", async ({
    page,
  }) => {
    await setup(page, ["overhang"]);
    await objectMenu(page, /^Auto orientation$|^Orientation automatique$/);
    await expect(msg(page)).toContainText("Orientation auto appliquée", {
      timeout: 60_000,
    });
  });
});

test.describe("menu d’un plateau", () => {
  test("Select All / Delete All / Add Models agissent sur le plateau @F:ctx.select-all @F:ctx.delete-all @F:ctx.add-models-3 @F:ctx.add-models-4", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await plateMenu(page, /^Select All$|^Tout sélectionner$/);
    await expect(
      page.locator('[data-testid="object-row"].bg-primary'),
    ).toHaveCount(2);
    await plateMenu(page, /^Delete All$|^Tout supprimer$/);
    await expect(rows(page)).toHaveCount(0);
    const chooser = page.waitForEvent("filechooser");
    await plateMenu(page, /^Add Models$|^Ajouter des modèles$/);
    await (await chooser).setFiles(fixture("cube20"));
    await expect(rows(page)).toHaveCount(1, { timeout: 60_000 });
  });

  test("Select All Plates sélectionne les objets de tous les plateaux @F:ctx.select-all-plates", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByRole("button", { name: "Ajouter un plateau" }).click();
    // R1 (Orca select_plate) : le plateau ajouté devient actif ; revenir au plateau 1.
    await page.getByTestId("plate-tab").nth(0).getByRole("button").first().click();
    // Le second objet part sur le plateau 2.
    await page
      .getByLabel(/^Plateau de /)
      .nth(1)
      .selectOption({ index: 1 });
    await expect(rows(page)).toHaveCount(1);
    await plateMenu(
      page,
      /^Select All Plates$|^Sélectionner tous les plateaux$/,
    );
    // Suppr agit sur toute la sélection : les deux plateaux sont vidés.
    await page.locator("body").focus();
    await page.keyboard.press("Delete");
    const tabs = page.getByTestId("plate-tab");
    await expect(tabs.nth(0)).toContainText("(0)");
    await expect(tabs.nth(1)).toContainText("(0)");
  });

  test("Arrange / Auto Rotate / Reload All opèrent sur le plateau @F:ctx.arrange @F:ctx.auto-rotate @F:ctx.reload-all", async ({
    page,
  }) => {
    await setup(page, ["cube20", "cylinder"]);
    await plateMenu(page, /^Arrange$|^Disposer$/);
    await expect(msg(page)).toContainText(
      "Arrangement : 2 objet(s) repositionné(s)",
      {
        timeout: 60_000,
      },
    );
    await plateMenu(page, /^Auto Rotate$|^Rotation automatique$/);
    await expect(msg(page)).toContainText("Orientation auto : 2 objet(s)", {
      timeout: 60_000,
    });
    await plateMenu(page, /^Reload All$|^Tout recharger$/);
    await expect(msg(page)).toContainText("rechargés depuis le disque");
    await ready(page, 2);
  });

  test("Delete Plate supprime le plateau visé @F:ctx.delete-plate @F:ctx.delete-plate-2", async ({
    page,
  }) => {
    await setup(page);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByRole("button", { name: "Ajouter un plateau" }).click();
    await expect(page.getByTestId("plate-tab")).toHaveCount(2);
    await plateMenu(page, /^Delete Plate$|^Supprimer le plateau$/, 1);
    await expect(page.getByTestId("plate-tab")).toHaveCount(1);
    await expect(rows(page)).toHaveCount(1);
  });
});

test.describe("menu d’un filament", () => {
  test("Edit ouvre les réglages ; Delete retire le slot @F:ctx.edit @F:ctx.delete-3", async ({
    page,
  }) => {
    await setup(page);
    await page.getByTestId("add-filament-slot").click();
    const slots = page.getByTestId("filament-slot");
    await expect(slots).toHaveCount(2);
    await slots.nth(1).click({ button: "right" });
    await menuItem(page, /^Edit$|^Modifier$|^Éditer$/);
    await expect(page.getByTestId("settings-dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("settings-dialog")).toBeHidden();
    await slots.nth(1).click({ button: "right" });
    await menuItem(page, /^Delete$|^Supprimer$/);
    await expect(slots).toHaveCount(1);
  });
});
