// Volumes : gizmo 3D (déplacer) sur le volume sélectionné, appliqué dans le
// repère de l'objet, une seule étape d'historique par geste, persisté dans
// objects[].volumes[]. L'ancre écran du gizmo est exposée par la scène
// (data-volume-gizmo-anchor = "x,y" en px CSS relatifs au canvas).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

type Volume = {
  id: string;
  offset: number[];
  size: number[];
  rotation?: number[];
};
type Project = { scene: { objects: { volumes?: Volume[] }[] } };

async function saveAndRead(page: Page, id: string): Promise<Volume> {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status()).toBeLessThan(300);
  const project = (await (
    await page.request.get(`/api/projects/${id}`)
  ).json()) as Project;
  return project.scene.objects[0].volumes![0];
}

test("volume : gizmo de déplacement, un pas d’historique, persistance @F:editor.volume-gizmo", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("object-row").first().click({ button: "right" });
  await page
    .getByRole("menuitem", {
      name: /Add negative volume|Ajouter un volume négatif/i,
    })
    .click();
  await page.getByTestId("object-volume-select").first().click();

  // Déplacer est l'outil initial : ne cliquer que s'il n'est pas déjà actif,
  // car un second clic sur un outil de transformation le désactive.
  const move = page.getByTestId("tool-move");
  if ((await move.getAttribute("aria-pressed")) !== "true") await move.click();
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-volume-gizmo", "translate");
  await expect(canvas).toHaveAttribute(
    "data-volume-gizmo-anchor",
    /^\d+(\.\d+)?,\d+(\.\d+)?$/,
  );
  const [ax, ay] = (await canvas.getAttribute("data-volume-gizmo-anchor"))!
    .split(",")
    .map(Number);
  const box = (await canvas.boundingBox())!;
  const x = box.x + ax;
  const y = box.y + ay;

  // Geste unique avec pauses > 400 ms (débounce de l'historique) : poignée
  // centrale du gizmo (déplacement dans le plan de vue).
  await page.mouse.move(x, y);
  await page.mouse.down();
  for (let i = 1; i <= 4; i++) {
    await page.mouse.move(x + i * 25, y, { steps: 5 });
    await page.waitForTimeout(500);
  }
  await page.mouse.up();
  await page.waitForTimeout(600);

  const moved = await saveAndRead(page, id);
  expect(Math.hypot(...moved.offset)).toBeGreaterThan(1);
  expect(moved.size).toEqual([10, 10, 10]);

  // Un seul Ctrl+Z annule tout le geste.
  await page.keyboard.press("Control+z");
  const undone = await saveAndRead(page, id);
  expect(undone.offset).toEqual([0, 0, 0]);
});
