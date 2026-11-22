// Phase 31 — /slice reçoit la scène affichée (lot 29, sans enregistrement
// préalable), notifications répétées qui réapparaissent, aperçu G-code :
// position de l'outil au curseur horizontal (ToolPosition d'Orca), « Aller à la
// couche » (Maj+G), plateaux dans l'aperçu, statistiques d'un G-code externe.
import { expect, test, type Page } from "@playwright/test";
import { writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { bootstrap, importModel, sliceInUi } from "../lib/app";

test.use({ locale: "fr-FR" });

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

test("trancher envoie la scène affichée sans enregistrer ; aperçu : outil, aller à la couche @F:core.slice-ui @F:shortcut.home @F:shortcut.end", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const projectId = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  // Modification non enregistrée : objet déplacé au clavier (la touche A
  // passe désormais par l'arrangeur, qui enregistre avant de disposer).
  await page.getByTestId("object-name").first().click();
  await focusEditor(page);
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("ArrowUp");
  const saves: string[] = [];
  const bodies: Record<string, unknown>[] = [];
  page.on("request", (req) => {
    const url = req.url();
    if (
      url.endsWith(`/api/projects/${projectId}/slice`) &&
      req.method() === "POST"
    )
      bodies.push(req.postDataJSON() as Record<string, unknown>);
    else if (
      url.includes(`/api/projects/${projectId}`) &&
      ["PUT", "PATCH"].includes(req.method())
    )
      saves.push(`${req.method()} ${url}`);
  });
  const layers = await sliceInUi(page);
  expect(layers).toBeGreaterThan(5);
  expect(bodies).toHaveLength(1);
  expect(typeof bodies[0].scene).toBe("object");
  expect(saves).toEqual([]);

  const canvas = page.getByTestId("preview-canvas");
  await expect(canvas).toBeVisible({ timeout: 60_000 });
  // Position de l'outil : fin de la couche haute, Home / → / End.
  const tool = page.getByTestId("preview-tool-position");
  await expect(tool).toBeVisible();
  const end = Number(await tool.getAttribute("data-index"));
  expect(end).toBeGreaterThan(0);
  await expect(page.getByTestId("preview-scene")).toHaveAttribute(
    "data-tool-marker",
    /^-?\d/,
  );
  await focusEditor(page);
  await page.keyboard.press("Home");
  await expect(canvas).toHaveAttribute("data-move-cursor", "0");
  await expect(tool).toHaveAttribute("data-index", "-1");
  await page.keyboard.press("ArrowRight");
  await expect(canvas).toHaveAttribute("data-move-cursor", "1");
  await expect(tool).toHaveAttribute("data-index", "0");
  await page.keyboard.press("End");
  await expect(tool).toHaveAttribute("data-index", String(end));
  // Propriétés dépliables (type de ligne, vitesse, débit…).
  await page.keyboard.press("End");
  await page.getByTestId("preview-tool-toggle").click();
  await expect(page.getByTestId("preview-tool-properties")).toContainText(
    "Débit",
  );
  // Maj+G : aller à la couche 3.
  await focusEditor(page);
  await page.keyboard.press("Shift+G");
  const form = page.getByTestId("preview-goto-layer");
  await expect(form).toBeVisible();
  await form.getByRole("spinbutton").fill("3");
  await form.getByRole("spinbutton").press("Enter");
  await expect(canvas).toHaveAttribute("data-layer-high", "2");
  await expect(form).toHaveCount(0);
});

test("un même message répété réapparaît (nouvelle notification) @F:toolbar.arrange", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  const msg = page.getByTestId("editor-message");
  await page.getByTestId("toolbar-arrange").click();
  await page.getByTestId("arrange-all").click();
  await expect(msg).toContainText("Arrangement", { timeout: 30_000 });
  const text = (await msg.textContent())!.trim();
  const seq = Number(await msg.getAttribute("data-seq"));
  await expect(msg).toHaveAttribute("data-faded", "true", { timeout: 10_000 });
  await page.getByTestId("arrange-all").click();
  await expect(msg).toHaveAttribute("data-seq", String(seq + 1), {
    timeout: 30_000,
  });
  await expect(msg).not.toHaveAttribute("data-faded", "true");
  await expect(msg).toHaveText(text);
});

test("G-code externe : statistiques du pied de fichier ; plateaux listés dans l’aperçu @F:editor.menu-open-gcode", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await bootstrap(page);
  const lines = ["G28", "G90", "M83"];
  for (let l = 0; l < 4; l++) {
    const z = (0.2 * (l + 1)).toFixed(2);
    lines.push(
      ";LAYER_CHANGE",
      `;Z:${z}`,
      `G1 Z${z} F600`,
      ";TYPE:Outer wall",
      ";WIDTH:0.45",
      ";HEIGHT:0.2",
    );
    lines.push(
      "G1 X100 Y100 F3000",
      "G1 X120 Y100 E1 F1800",
      "G1 X120 Y120 E1",
      "G1 X100 Y120 E1",
    );
  }
  lines.push(
    "; filament used [mm] = 120.5",
    "; filament used [g] = 0.36",
    "; total filament used [g] = 0.36",
    "; filament cost = 0.01",
    "; total layers count = 4",
    "; estimated printing time (normal mode) = 1h 2m 3s",
  );
  const dir = mkdtempSync(join(tmpdir(), "p31-"));
  const file = join(dir, "externe.gcode");
  writeFileSync(file, lines.join("\n") + "\n");
  await page.getByTestId("gcode-open-input").setInputFiles(file);
  await expect(page.getByTestId("editor-message")).toContainText(
    "externe.gcode",
    { timeout: 30_000 },
  );
  await expect(page.getByTestId("preview-canvas")).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.getByTestId("slice-summary")).toContainText("1h 2m 3s");
  // Deux plateaux : puces de plateau dans la colonne de l'aperçu.
  await page
    .getByRole("complementary", { name: "Configuration" })
    .getByRole("button", { name: "Objets", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Ajouter un plateau" })
    .first()
    .click();
  await expect(page.getByTestId("preview-plate")).toHaveCount(2);
});
