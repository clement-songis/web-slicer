// Historique de scène (Ctrl+Z / Ctrl+Y / Ctrl+Maj+Z, parité Orca) et renommage
// en ligne des objets et plateaux, persistés à l'enregistrement.
import { expect, test } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

test("Édition › Annuler / Rétablir : suppression puis restauration de l’objet @F:editor.undo-redo", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  const rows = page.getByTestId("object-row");
  await expect(rows).toHaveCount(2);
  // Laisse l'historique enregistrer l'état « 2 objets » (regroupement 400 ms).
  await page.waitForTimeout(800);

  await page.getByRole("button", { name: /^Supprimer cylinder/ }).click();
  await expect(rows).toHaveCount(1);
  await page.waitForTimeout(800);

  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+z");
  await expect(rows).toHaveCount(2);
  await expect(
    page.getByTestId("object-name").filter({ hasText: "cylinder" }),
  ).toBeVisible();

  await page.keyboard.press("Control+y");
  await expect(rows).toHaveCount(1);
  await page.keyboard.press("Control+z");
  await expect(rows).toHaveCount(2);
  await page.keyboard.press("Control+Shift+z");
  await expect(rows).toHaveCount(1);
});

test("Objets › Renommer un objet et un plateau : persisté après rechargement @F:editor.rename", async ({
  page,
}) => {
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });

  await page.getByTestId("object-name").first().dblclick();
  const input = page.getByTestId("object-rename-input");
  await input.fill("Ma pièce");
  await input.press("Enter");
  await expect(page.getByTestId("object-name").first()).toContainText(
    "Ma pièce",
  );

  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page
    .getByTestId("plate-tab")
    .first()
    .getByRole("button")
    .first()
    .dblclick();
  const plateInput = page.getByTestId("plate-rename-input");
  await plateInput.fill("Plateau A");
  await plateInput.press("Enter");
  await expect(page.getByTestId("plate-tab").first()).toContainText(
    "Plateau A",
  );

  await page.getByTestId("save-project").click();
  await expect(page.getByTestId("save-project")).toBeEnabled({
    timeout: 30_000,
  });
  await page.waitForTimeout(1000);
  await page.reload();
  await expect(page.getByTestId("object-name").first()).toContainText(
    "Ma pièce",
    { timeout: 60_000 },
  );
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await expect(page.getByTestId("plate-tab").first()).toContainText(
    "Plateau A",
  );
});
