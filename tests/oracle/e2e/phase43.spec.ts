// Phase 43 — corrections de l'audit 42 (navigateur), partie interface : objets
// hors plateau (I1), 3MF multi-plateaux importé (I1), « Quitter sans
// enregistrer » (C3), perte de réseau pendant un import (C1), étiquettes de
// plateau sous les dialogues (I4), clic droit dans la peinture (S2), Tab et
// focus (A1), menus au clavier et langue du document (A2/A3), cosmétiques.
//
// Oracle Orca 2.4.1 : GLVolumeCollection::check_outside_state + gouraud.fs
// (« Following objects are laid over the boundary of plate… », Trancher grisé),
// GLGizmoPainterBase::gizmo_event (droit = bloquer, Maj = effacer),
// GLCanvas3D::on_key (Tab capté par le canevas), menus natifs (Alt+lettre).
import { expect, test, type Page, type Request } from "@playwright/test";
import { join } from "node:path";
import { REPO, bootstrap, importModel } from "../lib/app";

type Msg = Record<string, unknown>;

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  return id;
}
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
async function anchor(page: Page): Promise<[number, number]> {
  const canvas = page.getByTestId("scene-canvas");
  await expect(canvas).toHaveAttribute("data-object-anchor", /^\d+,\d+$/);
  const [x, y] = (await canvas.getAttribute("data-object-anchor"))!.split(",").map(Number);
  const box = (await canvas.boundingBox())!;
  return [box.x + x, box.y + y];
}
/** Point de la scène sans objet (coin bas gauche du canevas). */
async function emptySceneClick(page: Page) {
  const box = (await page.getByTestId("scene-canvas").boundingBox())!;
  await page.mouse.click(box.x + box.width - 60, box.y + box.height - 40);
}

test.describe("audit 42 (FR)", () => {
  test.use({ locale: "fr-FR" });

  // ─── I1 : objet à cheval / hors plateau ─────────────────────────────────
  test("I1 : objet à cheval puis hors plateau — signalé, drapeau off_bed, Trancher grisé ; replacé → tranchable @F:core.slice-ui @F:gizmo.move @F:core.project-save", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    const canvas = page.getByTestId("scene-canvas");
    const slice = page.getByTestId("slice-button");
    await expect(slice).toBeEnabled();
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    // Voron 350 : cube de 20 centré en X = 345 → déborde de 5 mm.
    await page.getByTestId("transform-position-x").fill("345");
    await expect(canvas).toHaveAttribute("data-outside-objects", /:partly$/);
    await expect(slice).toBeDisabled();
    await expect(
      page.getByTestId("validation-notice").filter({ hasText: "dépassent les limites de la plaque" }),
    ).toHaveCount(1);
    await expect(canvas).toHaveAttribute("data-issue-objects", /:error/);
    // Entièrement dehors (X = 500) : signalé et drapeau off_bed enregistré.
    await page.getByTestId("transform-position-x").fill("500");
    await expect(canvas).toHaveAttribute("data-outside-objects", /:outside$/);
    await expect(page.getByTestId("validation-notice").filter({ hasText: "est hors du plateau" })).toHaveCount(1);
    await saveProject(page);
    const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as {
      scene: { objects: { off_bed?: boolean }[] };
    };
    expect(project.scene.objects[0].off_bed).toBe(true);
    // Raccourci Ctrl+R : jamais de tranchage avec un objet dehors.
    let slicePosts = 0;
    page.on("request", (r) => {
      if (r.method() === "POST" && /\/slice$/.test(r.url())) slicePosts++;
    });
    await emptySceneClick(page);
    await page.keyboard.press("Control+r");
    await page.waitForTimeout(1_000);
    expect(slicePosts).toBe(0);
    // Replacé : tranchable, plus de drapeau (l'outil Échelle est resté ouvert).
    await page.getByTestId("object-name").first().click();
    if (!(await page.getByTestId("transform-position-x").isVisible())) await page.getByTestId("tool-scale").click();
    await page.getByTestId("transform-position-x").fill("175");
    await expect(canvas).not.toHaveAttribute("data-outside-objects", /./);
    await expect(slice).toBeEnabled();
    await expect(page.getByTestId("validation-notice").filter({ hasText: "hors du plateau" })).toHaveCount(0);
  });

  // ─── I1 : 3MF à deux plateaux importé ──────────────────────────────────
  test("I1 : « Importer » un 3MF à deux plateaux — chaque objet sur son plateau, dans le plateau @F:ui.import-3mf-multi @F:core.import", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await bootstrap(page);
    await page
      .getByTestId("import-input")
      .setInputFiles(join(REPO, "engine", "tests", "fixtures", "gcode_3mf", "two_plates.3mf"));
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 90_000 });
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    const tabs = page.getByTestId("plate-tab");
    await expect(tabs).toHaveCount(2);
    await expect(tabs.nth(0)).toContainText("(1)");
    await expect(tabs.nth(1)).toContainText("(1)");
    await expect(page.getByTestId("scene-canvas")).not.toHaveAttribute("data-outside-objects", /./);
    await expect(page.getByTestId("slice-button")).toBeEnabled();
  });

  // ─── C3 : quitter sans enregistrer ─────────────────────────────────────
  test("C3 : quitter sans enregistrer rouvre exactement l'état enregistré (import, plateau, déplacement) @F:core.project-save @F:core.import", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    const savedX = await numberValue(page, "transform-position-x");
    await saveProject(page);
    // Modifications abandonnées : déplacement, nouveau plateau, import.
    await page.getByTestId("transform-position-x").fill(String(savedX - 40));
    await page.getByTestId("toolbar-addplate").click();
    await importModel(page, "cylinder");
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
    await expect(page.getByTestId("save-project")).toHaveAttribute("data-unsaved", "true");
    page.on("dialog", (d) => void d.accept());
    await page.reload();
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
    await page.waitForTimeout(2_000);
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(page.getByTestId("object-name").first()).toContainText("cube20.stl");
    await expect(page.getByTestId("plate-tab")).toHaveCount(1);
    await expect(page.getByTestId("save-project")).toHaveAttribute("data-unsaved", "false");
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    expect(await numberValue(page, "transform-position-x")).toBeCloseTo(savedX, 2);
  });

  // ─── C1 : réseau coupé pendant l'import ───────────────────────────────
  test("C1 : liaison perdue pendant le chargement du maillage — message juste, reprise automatique @F:core.import", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await bootstrap(page);
    let cut = true;
    await page.route("**/api/models/*/mesh", (route) => (cut ? route.abort("internetdisconnected") : route.continue()));
    await page.getByTestId("import-input").setInputFiles(join(REPO, "tests", "oracle", "fixtures", "cube20.stl"));
    await expect(page.getByText(/Connexion perdue pendant l'import de « cube20\.stl »/)).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(page.getByText("en attente de la connexion…").first()).toBeVisible();
    // Retour de la liaison : la sonde la détecte, le maillage est rechargé.
    cut = false;
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(page.getByTestId("scene-canvas")).toHaveAttribute("data-object-anchors", /./, { timeout: 30_000 });
    await saveProject(page);
  });

  // ─── S2 : clic droit dans la peinture des supports ─────────────────────
  test("S2 : peinture des supports — clic droit = bloquer (pas de menu contextuel) @F:gizmo.fdmsupports @F:editor.paint-supports", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const sent: Msg[] = [];
    page.on("websocket", (ws) => {
      if (!/\/paint$/.test(ws.url())) return;
      ws.on("framesent", (f) => {
        try {
          sent.push(JSON.parse(String(f.payload)));
        } catch {
          /* binaire */
        }
      });
    });
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-support-paint").click();
    await expect(page.getByTestId("paint-mouse-hint")).toContainText("clic droit : bloquer");
    const [x, y] = await anchor(page);
    await page.mouse.move(x, y);
    await page.mouse.down({ button: "right" });
    await page.mouse.move(x + 4, y + 2, { steps: 3 });
    await page.mouse.up({ button: "right" });
    await expect(page.getByRole("menu")).toHaveCount(0);
    await expect
      .poll(() => sent.filter((m) => JSON.stringify(m).includes('"state":2')).length, { timeout: 20_000 })
      .toBeGreaterThan(0);
  });

  // ─── A1 : Tab et focus ─────────────────────────────────────────────────
  test("A1 : Tab entre dans la page au chargement ; ne bascule Préparer/Aperçu que depuis la scène @F:shortcut.tab", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await ready(page, ["cube20"]);
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    // Clic dans un panneau : pas la scène.
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    expect(await page.evaluate(() => document.activeElement?.tagName)).not.toBe("BODY");
    // Clic dans la scène (zone vide) : Tab bascule vers l'Aperçu, puis revient.
    await emptySceneClick(page);
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("scene-canvas")).toBeHidden();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
  });

  // ─── A2 / A3 : menus au clavier, langue du document ───────────────────
  test("A2/A3 : Alt+F ouvre Fichier, flèches, Échap rend le focus ; <html lang=fr> @F:menu.top", async ({ page }) => {
    test.setTimeout(120_000);
    await ready(page, []);
    expect(await page.evaluate(() => document.documentElement.lang)).toBe("fr");
    await emptySceneClick(page);
    await page.keyboard.press("Alt+f");
    const menu = page.getByRole("menu");
    await expect(menu).toHaveCount(1);
    const first = await page.evaluate(() => document.activeElement?.textContent?.trim() ?? "");
    expect(first).toMatch(/Nouveau projet/);
    await page.keyboard.press("ArrowDown");
    const second = await page.evaluate(() => document.activeElement?.textContent?.trim() ?? "");
    expect(second).not.toBe(first);
    await page.keyboard.press("ArrowUp");
    expect(await page.evaluate(() => document.activeElement?.textContent?.trim() ?? "")).toBe(first);
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("button", { name: /^Édition$/ })).toHaveAttribute("aria-expanded", "true");
    await page.keyboard.press("Escape");
    await expect(menu).toHaveCount(0);
    await expect(page.getByRole("button", { name: /^Édition$/ })).toBeFocused();
  });

  // ─── I4 : étiquettes de plateau sous les dialogues (1280×720) ──────────
  test("I4 : 1280×720 — les étiquettes de plateau ne passent pas au-dessus d'un dialogue @F:toolbar.addplate @F:menu.preferences", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 1280, height: 720 });
    await ready(page, ["cube20"]);
    await page.getByTestId("toolbar-addplate").click();
    await expect(page.getByTestId("scene-plate-label")).toHaveCount(2);
    await page
      .getByRole("button", { name: /^Fichier$/ })
      .first()
      .click();
    await page
      .getByRole("menu")
      .getByRole("menuitem", { name: /Préférences/ })
      .click();
    await expect(page.getByTestId("preferences-dialog")).toBeVisible();
    const covered = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('[data-testid="scene-plate-label"]')].map((label) => {
        const r = label.getBoundingClientRect();
        const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        return !!hit && (hit === label || label.contains(hit));
      }),
    );
    expect(covered).toEqual([false, false]);
  });

  // ─── Cosmétiques ───────────────────────────────────────────────────────
  test("cosmétiques : pouces « 0,7874 », Fichiers récents d'un compte à un projet, conflit avec « Écraser » @F:menu.preferences @F:menu.open-project @F:core.project-save", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    await saveProject(page);
    // V-a : le projet ouvert figure dans Fichiers récents.
    const name = ((await (await page.request.get(`/api/projects/${id}`)).json()) as { name: string }).name;
    await page
      .getByRole("button", { name: /^Fichier$/ })
      .first()
      .click();
    const recent = page.getByRole("menu").getByRole("button", { name: /^Fichiers récents/ });
    await recent.hover();
    await expect(page.getByRole("menu").getByRole("menuitem", { name })).toBeVisible();
    await page.keyboard.press("Escape");
    // V-b : pouces au format français.
    await page
      .getByRole("button", { name: /^Fichier$/ })
      .first()
      .click();
    await page
      .getByRole("menu")
      .getByRole("menuitem", { name: /Préférences/ })
      .click();
    await page.getByTestId("preferences-dialog").getByTestId("pref-units").selectOption("in");
    await page.keyboard.press("Escape");
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    await expect(page.getByTestId("transform-size-x")).toHaveValue("0,7874");
    await page.getByTestId("transform-size-x").fill("1,5");
    await expect.poll(() => numberValue(page, "transform-size-z")).toBeCloseTo(1.5, 3);
    // C2 : écriture concurrente → « Recharger » / « Écraser ».
    const current = (await (await page.request.get(`/api/projects/${id}`)).json()) as Record<string, unknown>;
    const bump = await page.request.put(`/api/projects/${id}`, {
      data: {
        scene: current.scene,
        active_presets: current.active_presets,
        expected_version: Number(current.version),
      },
    });
    expect(bump.status(), await bump.text()).toBeLessThan(300);
    await page.getByTestId("save-project").click();
    await expect(page.getByTestId("save-conflict")).toContainText("modifié ailleurs");
    await expect(page.getByTestId("save-conflict-reload")).toBeVisible();
    await page.getByTestId("save-conflict-overwrite").click();
    await expect(page.getByTestId("save-project")).toHaveAttribute("data-unsaved", "false", { timeout: 15_000 });
    await expect(page.getByTestId("save-conflict")).toHaveCount(0);
  });
});
