// Phase 42 — corrections de l'audit 41 (navigateur) : résultat de tranchage
// restauré à la réouverture seulement s'il correspond au plateau (V1), objet
// reposé sur le plateau après échelle / rotation depuis le panneau (V2),
// Suppr dans la mesure (T1), T sur un texte existant = édition (T4), nouveau
// plateau actif (R1), URL d'hôte invalide refusée (M1) et cosmétiques.
//
// Oracle Orca 2.4.1 : Plater::priv::load_files (aucun résultat périmé rouvert),
// GLCanvas3D::do_scale / do_rotate (« snaps object to buildplate »),
// GLGizmoMeasure (« Delete — Restart selection »), GLGizmoEmboss::on_set_state,
// Plater::priv::on_action_add_plate (select_plate), MainFrame « Recent files ».
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, downloadGcode, importModel, openParam, sliceInUi } from "../lib/app";

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  return id;
}
async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}
/** « Sauvegarder » (barre supérieure) : attend le PUT du projet. */
async function saveProject(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse((r) => r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url())),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
  await expect(page.getByTestId("save-project")).toHaveAttribute("data-unsaved", "false");
}
async function numberValue(page: Page, testId: string): Promise<number> {
  return Number((await page.getByTestId(testId).inputValue()).replace(",", "."));
}

test.describe("audit 41 (FR)", () => {
  test.use({ locale: "fr-FR" });

  // ─── V1 : réouverture → pas de G-code périmé ─────────────────────────────
  test("V1 : réouverture — résultat restauré s'il correspond au plateau, sinon Préparer sans export @F:core.export-gcode @F:core.project-save @F:core.slice-ui", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    await ready(page, ["cube20"]);
    await saveProject(page);
    await sliceInUi(page);
    const exportBtn = page.getByTestId("export-gcode-button");
    await expect(exportBtn).toBeEnabled();
    // Rechargement sans changement : le résultat (encore valable) est rouvert.
    await page.reload();
    await expect(page.getByTestId("preview-view")).toBeVisible({ timeout: 60_000 });
    await expect(exportBtn).toBeEnabled({ timeout: 30_000 });
    // wall_loops modifié et enregistré : le G-code ne correspond plus au projet.
    await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
    const control = await openParam(page, "print", "Strength", "wall_loops");
    await expect(page.getByTestId("settings-dialog").locator('fieldset[aria-busy="true"]')).toHaveCount(0);
    await expect(control).not.toHaveValue("");
    const before = Number(await control.inputValue());
    await control.fill(String(before + 2));
    await control.press("Tab");
    await page.keyboard.press("Escape");
    await expect(exportBtn).toBeDisabled({ timeout: 5_000 });
    await saveProject(page);
    await page.reload();
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
    // Laisse le temps à la restauration (liste des jobs) : elle doit s'abstenir.
    await page.waitForTimeout(4_000);
    await expect(page.getByTestId("preview-view")).toHaveCount(0);
    await expect(exportBtn).toBeDisabled();
    await expect(page.getByTestId("send-to-printer")).toBeDisabled();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await expect(page.getByTestId("plate-tab").first()).toHaveAttribute("data-sliced", "false");
  });

  // ─── V2 : échelle / rotation depuis le panneau → reposé sur le plateau ───
  test("V2 : échelle (% et mm) et rotation depuis le panneau reposent l'objet ; G-code de 30 mm @F:gizmo.scale @F:param.scale @F:gizmo.rotate @F:param.rotate", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    expect(await numberValue(page, "transform-position-z")).toBeCloseTo(10, 2);
    await page.getByTestId("transform-scale-x").fill("150");
    await expect(page.getByTestId("transform-size-z")).toHaveValue("30");
    await expect.poll(() => numberValue(page, "transform-position-z")).toBeCloseTo(15, 2);
    await page.getByTestId("transform-size-x").fill("40");
    await expect.poll(() => numberValue(page, "transform-position-z")).toBeCloseTo(20, 2);
    await page.getByTestId("transform-size-x").fill("30");
    await expect.poll(() => numberValue(page, "transform-position-z")).toBeCloseTo(15, 2);
    const layers = await sliceInUi(page);
    const gcode = await downloadGcode(page);
    const zs = [...gcode.matchAll(/^;Z:([\d.]+)/gm)].map((m) => Number(m[1]));
    expect(Math.max(...zs)).toBeGreaterThan(29.7);
    expect(layers).toBeGreaterThan(140);
    // Rotation de 45° autour de X : posé sur l'arête (Z = 15 √2).
    await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-rotate").click();
    await page.getByTestId("transform-rotation-x").fill("45");
    await expect.poll(() => numberValue(page, "transform-position-z")).toBeCloseTo(15 * Math.SQRT2, 1);
  });

  // ─── T1 : Suppr dans l'outil Mesure ──────────────────────────────────────
  test("T1/T2 : Suppr dans l'outil Mesure recommence la mesure sans supprimer l'objet ; mode Face @F:gizmo.measure @F:shortcut.u @F:shortcut.del", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await focusEditor(page);
    await page.keyboard.press("u");
    const tool = page.getByTestId("measure-tool");
    await expect(tool).toBeVisible();
    await expect(tool).toContainText("Suppr : recommencer la sélection");
    await focusEditor(page);
    await page.keyboard.press("Delete");
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(tool).toBeVisible();
    await tool.getByTestId("measure-mode-face").click();
    await expect(tool.getByTestId("measure-mode-face")).toHaveAttribute("aria-checked", "true");
    await expect(tool).toContainText("Cliquez sur deux faces");
  });

  // ─── T4 / T3 / V4 : texte existant, placement, nom ─────────────────────
  test("T4/T3/V4 : texte nommé d'après son contenu, posé à côté du cube ; T le rouvre en édition @F:gizmo.emboss @F:shortcut.t @F:editor.emboss-text", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await focusEditor(page);
    await page.keyboard.press("Escape");
    await page.getByTestId("tool-emboss").click();
    await page.getByTestId("emboss-text").fill("Edit42");
    await page.getByTestId("emboss-submit").click();
    await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 120_000 });
    await expect(page.getByTestId("object-name").filter({ hasText: /(^|\s)Edit42$/ })).toHaveCount(1);
    await expect(page.getByTestId("overlap-warning")).toHaveCount(0);
    // Sélection du texte puis T : panneau en édition (pas un second texte).
    await page.getByTestId("object-name").filter({ hasText: /(^|\s)Edit42$/ }).click();
    await focusEditor(page);
    await page.keyboard.press("t");
    await expect(page.getByTestId("emboss-editing")).toBeVisible();
    await expect(page.getByTestId("emboss-text")).toHaveValue("Edit42");
  });

  // ─── R1 : nouveau plateau actif ─────────────────────────────────────────
  test("R1 : « Nouveau plateau » devient actif et reçoit l'import suivant @F:toolbar.addplate", async ({ page }) => {
    test.setTimeout(150_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("toolbar-addplate").click();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    const tabs = page.getByTestId("plate-tab");
    await expect(tabs).toHaveCount(2);
    await expect(tabs.nth(1)).toHaveAttribute("data-active", "true");
    await expect(tabs.nth(0)).toHaveAttribute("data-active", "false");
    await importModel(page, "cylinder");
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
    await expect(tabs.nth(1)).toContainText("(1)");
  });

  // ─── M1 : URL d'hôte invalide ───────────────────────────────────────────
  test("M1 : URL Moonraker invalide refusée avec message, champ conservé ; « Modifier l'URL » @F:core.moonraker", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await sliceInUi(page);
    await page.getByTestId("send-to-printer").click();
    const url = page.getByTestId("send-moonraker-url");
    await expect(url).toBeVisible();
    await url.fill("pas une url");
    await page.getByTestId("send-save-url").click();
    await expect(page.getByTestId("send-url-error")).toContainText("URL invalide");
    await expect(url).toHaveValue("pas une url");
    const printers = (await (await page.request.get("/api/printers")).json()) as { moonraker_url?: string | null }[];
    expect(printers.every((p) => !p.moonraker_url)).toBe(true);
    await url.fill("127.0.0.1:17125");
    await page.getByTestId("send-save-url").click();
    await expect(page.getByTestId("send-host-info")).toBeVisible();
    await page.getByTestId("send-edit-url").click();
    await expect(url).toHaveValue("http://127.0.0.1:17125");
    await page.getByTestId("send-edit-url-cancel").click();
    await expect(page.getByTestId("send-host-info")).toBeVisible();
  });

  // ─── Cosmétiques ───────────────────────────────────────────────────────
  test("cosmétiques : « Planaire », Fichier › Fichiers récents, Alt+N, Ctrl+K en un pas, pouces @F:gizmo.cut @F:menu.new-project @F:shortcut.ctrl-k @F:menu.preferences", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    // B4 : outil Coupe.
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-cut").click();
    await expect(page.getByTestId("cut-tool")).toBeVisible();
    await expect(page.getByTestId("cut-tool")).not.toContainText(/\bPlane\b/);
    await expect(page.getByTestId("cut-tool")).toContainText("Planaire");
    await page.getByTestId("tool-cut").click();
    // P2 / S1 : menu Fichier.
    const other = await page.request.post("/api/projects", { data: { name: "Récent 42" } });
    expect(other.status()).toBeLessThan(300);
    await page.getByRole("button", { name: /^Fichier$/ }).first().click();
    const menu = page.getByRole("menu");
    await expect(menu.getByRole("menuitem", { name: /Nouveau projet/ })).toContainText("Alt+N");
    const recent = menu.getByRole("button", { name: /^Fichiers récents/ });
    await recent.hover();
    await expect(recent).toHaveAttribute("aria-expanded", "true");
    await expect(menu.getByRole("menuitem", { name: "Récent 42" })).toBeVisible();
    await page.keyboard.press("Escape");
    // R2 : Ctrl+K (clone + arrangement) = une seule étape d'annulation.
    await page.getByTestId("object-name").first().click();
    await focusEditor(page);
    await page.keyboard.press("Control+k");
    await expect(page.getByTestId("object-row")).toHaveCount(2);
    await page.waitForTimeout(2_500);
    await focusEditor(page);
    await page.keyboard.press("Control+z");
    await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 5_000 });
    // P1 : unités en pouces dans le panneau de transformation.
    await page.getByRole("button", { name: /^Fichier$/ }).first().click();
    await page.getByRole("menu").getByRole("menuitem", { name: /Préférences/ }).click();
    const prefs = page.getByTestId("preferences-dialog");
    await prefs.getByTestId("pref-units").selectOption("in");
    await page.keyboard.press("Escape");
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    await expect(page.getByText("Taille (in)")).toBeVisible();
    expect(await numberValue(page, "transform-size-x")).toBeCloseTo(20 / 25.4, 3);
  });
});
