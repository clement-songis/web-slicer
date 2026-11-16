// BROUILLON (harness gelé) — à placer dans tests/oracle/e2e/ après dégel.
// Menu Calibration → dialogue → projet de calibration → tranchage → G-code :
// variation attendue couche par couche (Plater::calib_input_shaping_freq,
// Calib_Cornering, _calib_pa_pattern).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, downloadGcode, sliceInUi } from "../lib/app";

async function openCalib(page: Page, item: RegExp) {
  await page.getByRole("button", { name: "Calibration", exact: true }).click();
  await page.getByRole("menuitem", { name: item }).click();
  await expect(page.getByTestId("engine-calib-dialog")).toBeVisible();
}

async function createAndSlice(page: Page) {
  await page.getByTestId("engine-calib-confirm").click();
  await expect(page).toHaveURL(/\/projects\//, { timeout: 60_000 });
  await expect(page.getByTestId("object-row")).toHaveCount(1, {
    timeout: 60_000,
  });
  await sliceInUi(page);
  return downloadGcode(page);
}

const values = (gcode: string, prefix: string, key: string) =>
  gcode
    .split("\n")
    .filter((l) => l.startsWith(prefix))
    .map((l) => Number(l.split(key)[1]?.split(/[ ;]/)[0]))
    .filter(Number.isFinite);

test("input shaping fréquence : SET_INPUT_SHAPER par couche @F:editor.calib-input-shaping", async ({
  page,
}) => {
  await bootstrap(page);
  await openCalib(page, /input shaping frequency|input shaping : fréquence/i);
  await expect(page.getByTestId("calib-fixed")).toHaveValue("0.15");
  const freqs = values(
    await createAndSlice(page),
    "SET_INPUT_SHAPER",
    "SHAPER_FREQ_X=",
  );
  expect(freqs.length).toBeGreaterThan(20);
  expect(new Set(freqs).size).toBeGreaterThan(20);
});

test("virages : SQUARE_CORNER_VELOCITY ou M205 J par couche @F:editor.calib-cornering", async ({
  page,
}) => {
  await bootstrap(page);
  await openCalib(page, /cornering|virages/i);
  const gcode = await createAndSlice(page);
  const scv = values(
    gcode,
    "SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY",
    "VELOCITY=",
  );
  const jd = values(gcode, "M205 J", "J");
  expect(Math.max(new Set(scv).size, new Set(jd).size)).toBeGreaterThan(5);
});

test("PA motif : un motif par valeur de PA @F:editor.calib-pa-pattern", async ({
  page,
}) => {
  await bootstrap(page);
  await openCalib(page, /pressure advance pattern|pressure advance \(motif\)/i);
  await page.getByTestId("calib-end").fill("0.08");
  await page.getByTestId("calib-step").fill("0.02");
  const gcode = await createAndSlice(page);
  expect(gcode).toContain("; start pressure advance pattern for layer");
  const pa = values(gcode, "SET_PRESSURE_ADVANCE", "ADVANCE=").concat(
    values(gcode, "M900 K", "K"),
  );
  expect(
    new Set(pa.map((v) => Math.round(v * 1000))).size,
  ).toBeGreaterThanOrEqual(5);
});
