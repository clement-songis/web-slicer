// Régression UX : le menu contextuel long doit rester utilisable à 1280×720,
// y compris lorsque le clic droit est proche des bords du viewport.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

async function expectMenuInsideViewport(page: Page) {
  const menu = page.getByRole("menu");
  await expect(menu).toBeVisible();
  const box = await menu.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.y).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(1280);
  expect(box!.y + box!.height).toBeLessThanOrEqual(720);
}

test("menu objet long : repositionné et défilable en 1280×720", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await bootstrap(page);
  await importModel(page, "cube20");
  const row = page.getByTestId("object-row").first();

  await row.click({ button: "right" });
  await expectMenuInsideViewport(page);
  // Sans défilement, cette dernière entrée est hors du menu/viewport.
  const deleteItem = page.getByRole("menuitem", { name: /delete|supprimer/i });
  await deleteItem.scrollIntoViewIfNeeded();
  await expect(deleteItem).toBeInViewport();

  // Rouvre près du coin inférieur droit de la scène : le menu doit se retourner
  // vers la gauche/le haut plutôt que de sortir du viewport.
  await page
    .getByRole("button", { name: "Fermer le menu" })
    .click({ position: { x: 5, y: 5 } });
  const canvas = page.getByTestId("scene-canvas");
  const canvasBox = await canvas.boundingBox();
  expect(canvasBox).not.toBeNull();
  await canvas.click({
    button: "right",
    position: { x: canvasBox!.width - 5, y: canvasBox!.height - 5 },
  });
  await expectMenuInsideViewport(page);
  await deleteItem.scrollIntoViewIfNeeded();
  await expect(deleteItem).toBeInViewport();
  await deleteItem.click();
  await expect(page.getByTestId("object-row")).toHaveCount(0);
});
