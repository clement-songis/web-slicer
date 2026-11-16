// Parcours utilisateur complet, sans bootstrap de harness : un nouvel utilisateur
// venant d'Orca doit pouvoir aller de l'inscription à Moonraker sans connaître
// les identifiants internes des presets.
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  downloadGcode,
  fixture,
  openParam,
  setControl,
  sliceInUi,
  uniqueEmail,
} from "../lib/app";
import { configBlock } from "../lib/gcode";

const MOONRAKER = process.env.MOONRAKER_URL ?? "http://127.0.0.1:17125";
const CAPTURE_DIR = process.env.UX_CAPTURE_DIR;

async function selectMatching(page: Page, testid: string, pattern: RegExp) {
  const select = page.getByTestId(testid);
  await expect(select).toBeVisible();
  const options = await select.locator("option").evaluateAll((nodes) =>
    nodes.map((node) => ({
      value: (node as HTMLOptionElement).value,
      label: node.textContent ?? "",
    })),
  );
  const match =
    options.find((option) => pattern.test(option.label)) ??
    options.find((option) => option.value);
  expect(
    match,
    `${testid} : aucun preset compatible (${options.length} options)`,
  ).toBeDefined();
  await select.selectOption(match!.value);
  return match!.label;
}

test.use({ locale: "fr-FR" });

test("nouvel utilisateur : inscription → catalogue → STL → réglages → tranchage → export → Moonraker → bibliothèque", async ({
  page,
}) => {
  test.setTimeout(600_000);
  await page.setViewportSize({ width: 1440, height: 900 });
  const measurements: Record<string, unknown>[] = [];
  const capture = async (name: string) => {
    if (CAPTURE_DIR) {
      mkdirSync(CAPTURE_DIR, { recursive: true });
      await page.screenshot({
        path: join(CAPTURE_DIR, `${name}.png`),
        fullPage: true,
      });
    }
    for (const width of [1280, 1440, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      const size = await page.evaluate(() => ({
        documentWidth: document.documentElement.scrollWidth,
        viewportWidth: document.documentElement.clientWidth,
      }));
      measurements.push({ step: name, ...size });
      expect(
        size.documentWidth,
        `débordement horizontal à ${width}px sur ${name}`,
      ).toBeLessThanOrEqual(width);
      if (await page.getByTestId("editor-topbar").count()) {
        const projectTab = await page
          .getByRole("button", { name: "Projet", exact: true })
          .boundingBox();
        const importButton = await page
          .getByRole("button", { name: "Importer", exact: true })
          .boundingBox();
        expect(projectTab).not.toBeNull();
        expect(importButton).not.toBeNull();
        expect(
          projectTab!.x + projectTab!.width,
          `onglet Projet masqué à ${width}px sur ${name}`,
        ).toBeLessThanOrEqual(importButton!.x);
      }
      if (CAPTURE_DIR && width === 1280)
        await page.screenshot({
          path: join(CAPTURE_DIR, `${name}-1280.png`),
          fullPage: true,
        });
    }
    await page.setViewportSize({ width: 1440, height: 900 });
  };

  await test.step("inscription et choix de l’imprimante dans le catalogue", async () => {
    await page.goto("/register");
    await capture("01-register");
    await page.getByTestId("auth-email").fill(uniqueEmail());
    await page.getByTestId("auth-password").fill("harness-password-123");
    await page.getByTestId("auth-submit").click();
    await expect(page).toHaveURL(/\/setup$/);
    await page
      .getByPlaceholder("Rechercher une imprimante…")
      .fill("Voron 2.4 350");
    await page
      .getByRole("button", { name: /Voron 2\.4 350/ })
      .first()
      .click();
    await capture("02-printer-catalog");
    await page.getByRole("button", { name: "Confirmer" }).click();
    await expect(page).toHaveURL(/\/library$/);
    await expect(page.getByTestId("new-project")).toBeVisible();
    await capture("03-library-empty");
  });

  let projectId = "";
  await test.step("nouveau projet, import STL et profils compatibles présélectionnés", async () => {
    await page.getByTestId("new-project").click();
    await expect(page).toHaveURL(/\/projects\//);
    projectId = page.url().split("/").at(-1)!;
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    await capture("04-editor-empty");
    await page.getByTestId("import-input").setInputFiles(fixture("cube20"));
    await expect(page.getByTestId("object-row")).toHaveCount(1, {
      timeout: 60_000,
    });

    const nozzle = page.getByTestId("preset-select-printer");
    const filament = page.getByTestId("preset-select-filament");
    const process = page.getByTestId("preset-select-print");
    await expect(nozzle).toHaveValue(/.+/);
    await expect(filament).toHaveValue(/.+/);
    await expect(process).toHaveValue(/.+/);
    // Régression mesurée : 5914 filaments / 2882 process avant le filtre machine.
    expect(await filament.locator("option").count()).toBeLessThan(700);
    expect(await process.locator("option").count()).toBeLessThan(700);
    await selectMatching(page, "preset-select-filament", /Generic PLA/i);
    await selectMatching(page, "preset-select-print", /0\.20mm Standard/i);
    await expect(page.getByTestId("slice-button")).toBeEnabled();
    await expect(
      page.getByRole("button", { name: "Aperçu", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByTestId("slice-button").locator("span").first(),
    ).toHaveText("Trancher");
    await expect(
      page.getByTestId("export-gcode-button").locator("span").first(),
    ).toHaveText("G-code…");
    await capture("05-model-and-presets");
  });

  await test.step("modification d’un paramètre Orca", async () => {
    const control = await openParam(page, "print", "Strength", "wall_loops");
    expect(await setControl(control, "5")).toBe(true);
    await page.getByTestId("settings-apply").click();
    await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
    await capture("06-parameter-changed");
  });

  await test.step("tranchage, couches et coloration", async () => {
    const layerCount = await sliceInUi(page);
    expect(layerCount).toBeGreaterThan(1);
    const slider = page.getByTestId("layer-slider");
    await slider.fill(String(Math.floor(layerCount / 2)));
    await expect(slider).toHaveAttribute(
      "data-current-layer",
      String(Math.floor(layerCount / 2)),
    );
    const coloration = page.getByLabel("Coloration de l'aperçu");
    await coloration.selectOption("speed");
    await expect(page.getByTestId("preview-canvas")).toHaveAttribute(
      "data-coloration",
      "speed",
    );
    await coloration.selectOption("type");
    await expect(page.getByTestId("preview-canvas")).toHaveAttribute(
      "data-coloration",
      "type",
    );
    await expect(page.getByTestId("legend-item").first()).toBeVisible();
    await capture("07-preview");
  });

  await test.step("export du G-code modifié", async () => {
    const gcode = await downloadGcode(page);
    expect(configBlock(gcode).wall_loops).toBe("5");
    expect(gcode).toContain(";TYPE:");
  });

  await test.step("déclaration du Moonraker mock et envoi", async () => {
    const beforeResponse = await page.request.get(
      `${MOONRAKER}/server/files/list?root=gcodes`,
    );
    expect(beforeResponse.ok(), "Moonraker mock injoignable").toBe(true);
    const before = new Map(
      (
        (await beforeResponse.json()).result as {
          path: string;
          modified: number;
        }[]
      ).map((file) => [file.path, file.modified]),
    );
    await page.goto("/printers");
    await page.getByTestId("printer-add").click();
    await page.getByTestId("printer-moonraker-url").fill(MOONRAKER);
    await page.getByTestId("printer-save").click();
    await expect(
      page.getByTestId("printer-row").filter({ hasText: MOONRAKER }),
    ).toBeVisible();
    await capture("08-moonraker-configured");
    await page.goto(`/projects/${projectId}`);
    await expect(page.getByTestId("export-gcode-button")).toBeEnabled();
    await page.getByTestId("send-to-printer").click();
    await expect(page.getByTestId("send-printer-select")).toHaveValue(/.+/);
    await expect(page.getByTestId("send-printer-select")).toContainText(
      MOONRAKER,
    );
    await page.getByTestId("send-confirm").click();
    await expect(page.getByTestId("send-success")).toBeVisible({
      timeout: 60_000,
    });
    await capture("09-moonraker-sent");
    await expect
      .poll(async () => {
        const response = await page.request.get(
          `${MOONRAKER}/server/files/list?root=gcodes`,
        );
        const files = (await response.json()).result as {
          path: string;
          modified: number;
        }[];
        return files.filter(
          (file) =>
            file.path.endsWith(".gcode") &&
            (!before.has(file.path) || file.modified > before.get(file.path)!),
        ).length;
      })
      .toBeGreaterThan(0);
  });

  await test.step("retour à la bibliothèque avec le projet conservé", async () => {
    await page.goto("/library");
    await expect(page.getByTestId("new-project")).toBeVisible();
    await expect(
      page.locator(`a[href$="/projects/${projectId}"]`),
    ).toBeVisible();
    await capture("10-library-after");
    if (CAPTURE_DIR)
      writeFileSync(
        join(CAPTURE_DIR, "measurements.json"),
        JSON.stringify(measurements, null, 2),
      );
  });
});
