// Phase 40 — corrections de l'audit 40 (navigateur) : invalidation du G-code
// quand la scène change (B1), preset « (modifié) » (A2), Ctrl+Z limité à
// l'outil de coupe (B2), échelle en % + taille en mm (B3), interface anglaise
// sans français résiduel (C1/D3) et cosmétiques.
import { expect, test, type Locator, type Page } from "@playwright/test";
import { bootstrap, importModel, openParam, sliceInUi } from "../lib/app";

// ─── C1 / D3 : aucun français visible quand la langue est l'anglais ─────────

/** Indices de français : accents, guillemets français, mots-outils et termes d'interface. */
const FRENCH =
  /[éèêàçùûôîœ«»]|\b(le|la|les|des|du|une|et|pour|avec|sans|dans|sur|aucun|aucune|plateau|objets?|enregistrer|sauvegarder|envoyer|fermer|annuler|réglages|paramètres|créer|afficher|grouper|imprimante|projet|bibliothèque|couche|remplissage|parois?|modifié|ouvrir|supprimer|ajouter|terminé|choisir|nouveau|nouvelle|mot de passe|sélection|déplacements?|rétractions?|essuyages?|temps total|hérite)\b/i;
/** Libellés français attendus en anglais (nom de la langue dans le sélecteur, comme Orca). */
const ALLOWED = [/^Français$/, /Projet harness/];

async function frenchTexts(scope: Page | Locator): Promise<string[]> {
  const root = "locator" in scope && "goto" in scope ? scope.locator("body") : (scope as Locator);
  const texts = await root.evaluate((el) => {
    const out: string[] = [];
    for (const line of (el as HTMLElement).innerText.split("\n")) if (line.trim()) out.push(line.trim());
    for (const n of el.querySelectorAll("[title],[aria-label],[placeholder]")) {
      const h = n as HTMLElement;
      if (!h.offsetParent && h.tagName !== "BODY") continue;
      for (const a of ["title", "aria-label", "placeholder"]) {
        const v = h.getAttribute(a);
        if (v && v.trim()) out.push(v.trim());
      }
    }
    return out;
  });
  return [...new Set(texts)].filter((s) => FRENCH.test(s) && !ALLOWED.some((r) => r.test(s)));
}

async function expectNoFrench(scope: Page | Locator, where: string) {
  const fr = await frenchTexts(scope);
  expect(fr, `français visible (${where}) : ${JSON.stringify(fr.slice(0, 40))}`).toEqual([]);
}

test.describe("interface anglaise", () => {
  test.use({ locale: "en-US" });

  test("pages principales sans français en anglais (inscription, configuration, bibliothèque, éditeur, réglages, aperçu) @F:editor.menu-view-prefs @F:menu.preferences", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await page.goto("/register");
    await expect(page.locator("form")).toBeVisible();
    await expectNoFrench(page, "/register");
    await page.goto("/login");
    await expect(page.locator("form")).toBeVisible();
    await expectNoFrench(page, "/login");

    const id = await bootstrap(page);
    await expect(page.getByTestId("scene-canvas")).toBeVisible();
    await expectNoFrench(page, "éditeur vide");
    await importModel(page, "cube20");
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
    await expectNoFrench(page, "éditeur + objet");

    const dialog = page.getByTestId("settings-dialog");
    for (const scope of ["printer", "filament", "print"]) {
      await page.getByTestId(`open-settings-${scope}`).click();
      await expect(dialog).toBeVisible();
      await expectNoFrench(dialog, `réglages ${scope}`);
      await page.keyboard.press("Escape");
      await expect(dialog).toBeHidden();
    }

    await sliceInUi(page);
    await expectNoFrench(page, "aperçu");

    for (const path of ["/library", "/setup", "/printers", "/queue"]) {
      await page.goto(path);
      await page.waitForLoadState("networkidle");
      await expectNoFrench(page, path);
    }
    expect(id).toBeTruthy();
  });
});

test.describe("préférence de langue", () => {
test.use({ locale: "fr-FR" });
test("préférence de langue : tout bascule en anglais sans rechargement @F:menu.preferences @F:editor.menu-view-prefs", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await page.context().addInitScript(() => localStorage.removeItem("PARAGLIDE_LOCALE"));
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  await expect(prefs).toBeVisible();
  await prefs.getByTestId("pref-language").selectOption("en");
  await page.keyboard.press("Escape");
  await expect(prefs).toBeHidden();
  await expectNoFrench(page, "après Préférences › Langue = English");
  // Retour au français : l'interface rebascule.
  await page.keyboard.press("Control+p");
  await prefs.getByTestId("pref-language").selectOption("fr");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("save-project")).toContainText(/Sauvegarder|Enregistrer/);
});
});

// ─── B1 : G-code invalidé dès que la scène du plateau change ───────────────

test.describe("corrections fonctionnelles (FR)", () => {
  test.use({ locale: "fr-FR" });

  async function ready(page: Page, models: string[]) {
    await bootstrap(page);
    for (const m of models) await importModel(page, m);
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  }
  async function focusEditor(page: Page) {
    await page.locator("body").click({ position: { x: 5, y: 5 } });
    await page.locator("body").focus();
  }

  test("G-code périmé : suppression d'objet / annuler → Exporter, Envoyer et ✓ tombent jusqu'au prochain tranchage @F:core.export-gcode @F:editor.undo-redo @F:core.slice-ui", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20", "cylinder"]);
    await sliceInUi(page);
    const exportBtn = page.getByTestId("export-gcode-button");
    const send = page.getByTestId("send-to-printer");
    const tab = page.getByTestId("plate-tab").first();
    await expect(exportBtn).toBeEnabled();
    await expect(send).toBeEnabled();
    await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await expect(tab).toHaveAttribute("data-sliced", "true");
    // Supprimer le cylindre : le résultat ne correspond plus à la scène.
    await page.getByTestId("object-name").nth(1).click();
    await focusEditor(page);
    await page.keyboard.press("Delete");
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await expect(exportBtn).toBeDisabled({ timeout: 5_000 });
    await expect(send).toBeDisabled();
    await expect(tab).toHaveAttribute("data-sliced", "false");
    // Nouveau tranchage → de nouveau exportable ; annuler (le cylindre revient) → périmé.
    await sliceInUi(page);
    await expect(exportBtn).toBeEnabled();
    await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
    await focusEditor(page);
    await page.keyboard.press("Control+z");
    await expect(page.getByTestId("object-row")).toHaveCount(2);
    await expect(exportBtn).toBeDisabled({ timeout: 5_000 });
    await expect(send).toBeDisabled();
  });

  test("G-code périmé : un réglage modifié invalide aussi le résultat @F:core.export-gcode @F:core.slice-ui", async ({ page }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await sliceInUi(page);
    const exportBtn = page.getByTestId("export-gcode-button");
    await expect(exportBtn).toBeEnabled();
    const control = await openParam(page, "print", "Strength", "wall_loops");
    // Valeur lue après chargement du preset (sinon « » → 0 et la saisie peut
    // retomber sur la valeur du preset : aucune modification, pas de pastille).
    await expect(page.getByTestId("settings-dialog").locator('fieldset[aria-busy="true"]')).toHaveCount(0);
    await expect(control).not.toHaveValue("");
    const before = Number(await control.inputValue());
    await control.fill(String(before + 2));
    await control.press("Tab");
    await page.keyboard.press("Escape");
    await expect(exportBtn).toBeDisabled({ timeout: 5_000 });
  });

  // ─── A2 : preset modifié signalé ────────────────────────────────────────
  test("preset modifié : pastille « (modifié) » dans la barre latérale et l'en-tête du dialogue @F:ui.preset-save-as @F:ui.preset-management", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await ready(page, []);
    const select = page.getByTestId("preset-select-print");
    await expect(select).toHaveAttribute("data-modified", "false");
    await expect(page.getByTestId("preset-select-print-modified")).toHaveCount(0);
    const control = await openParam(page, "print", "Strength", "wall_loops");
    // Valeur lue après chargement du preset (sinon « » → 0 et la saisie peut
    // retomber sur la valeur du preset : aucune modification, pas de pastille).
    await expect(page.getByTestId("settings-dialog").locator('fieldset[aria-busy="true"]')).toHaveCount(0);
    await expect(control).not.toHaveValue("");
    const before = Number(await control.inputValue());
    await control.fill(String(before + 3));
    await control.press("Tab");
    const dialog = page.getByTestId("settings-dialog");
    await expect(dialog.getByTestId("preset-modified")).toHaveText("(modifié)");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(select).toHaveAttribute("data-modified", "true");
    await expect(page.getByTestId("preset-select-print-modified")).toBeVisible();
    await expect(page.getByTestId("preset-select-print-modified")).toHaveText("(modifié)");
  });

  // ─── B2 : Ctrl+Z dans l'outil de coupe ──────────────────────────────────
  test("outil Coupe : Ctrl+Z / Ctrl+Y n'annulent que les connecteurs, jamais l'import @F:gizmo.cut @F:shortcut.ctrl-z @F:shortcut.ctrl-y", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-cut").click();
    await expect(page.getByTestId("cut-tool")).toBeVisible();
    await page.getByTestId("cut-connectors-edit").click();
    const map = page.getByTestId("cut-section-map");
    const box = (await map.boundingBox())!;
    await page.mouse.click(box.x + box.width / 2 - 20, box.y + box.height / 2);
    await page.waitForTimeout(500);
    await page.mouse.click(box.x + box.width / 2 + 20, box.y + box.height / 2);
    const count = page.getByTestId("cut-connector-count");
    await expect(count).toHaveText("2");
    await page.waitForTimeout(500);
    await focusEditor(page);
    await page.keyboard.press("Control+z");
    await expect(count).toHaveText("1");
    await page.keyboard.press("Control+z");
    await expect(count).toHaveText("0");
    // Plus rien à annuler dans l'outil : la scène et l'outil restent intacts.
    await page.keyboard.press("Control+z");
    await expect(page.getByTestId("cut-tool")).toBeVisible();
    await expect(page.getByTestId("object-row")).toHaveCount(1);
    await page.keyboard.press("Control+y");
    await expect(count).toHaveText("1");
    await page.keyboard.press("Control+y");
    await expect(count).toHaveText("2");
  });

  // ─── B3 : échelle en %, taille en mm, verrou ───────────────────────────
  test("outil Échelle : pourcentages (100 %), taille en mm, verrou de proportions @F:gizmo.scale @F:param.scale", async ({
    page,
  }) => {
    test.setTimeout(150_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("object-name").first().click();
    await page.getByTestId("tool-scale").click();
    const sx = page.getByTestId("transform-scale-x");
    await expect(sx).toHaveValue("100");
    await expect(page.getByTestId("transform-size-x")).toHaveValue("20");
    await sx.fill("150");
    await expect(page.getByTestId("transform-scale-z")).toHaveValue("150");
    await expect(page.getByTestId("transform-size-y")).toHaveValue("30");
    // Taille en mm, proportions verrouillées : 40 mm → 200 % partout.
    await page.getByTestId("transform-size-x").fill("40");
    await expect(page.getByTestId("transform-scale-y")).toHaveValue("200");
    // Verrou ouvert : un seul axe.
    await page.getByTestId("transform-uniform-scale").click();
    await page.getByTestId("transform-scale-z").fill("50");
    await expect(page.getByTestId("transform-scale-x")).toHaveValue("200");
    await expect(page.getByTestId("transform-size-z")).toHaveValue("10");
  });

  // ─── B5 : hôte injoignable dans l'onglet Appareil ──────────────────────
  test("onglet Appareil : hôte injoignable → message d'erreur au lieu d'un cadre blanc @F:ui.device", async ({ page }) => {
    test.setTimeout(120_000);
    await bootstrap(page);
    const existing = await (await page.request.get("/api/printers")).json();
    const created = await page.request.post("/api/printers", {
      data: {
        name: "OctoPi injoignable",
        moonraker_url: "http://127.0.0.1:9",
        host_type: "octoprint",
        api_key: "CLE",
        machine_preset_id: existing[0].machine_preset_id,
      },
    });
    expect(created.status()).toBe(201);
    await page.getByRole("button", { name: /^(appareil|device)$/i }).first().click();
    const panel = page.getByTestId("device-panel");
    await panel.getByText("OctoPi injoignable").first().click();
    const webui = panel.getByTestId("device-webui");
    await expect(webui).toHaveAttribute("data-webui-state", "link:unreachable", { timeout: 15_000 });
    await expect(webui.getByTestId("device-webui-fallback")).toContainText("injoignable");
  });

  // ─── D2 : sous-menus au survol ─────────────────────────────────────────
  test("barre de menus : le sous-menu Exporter s'ouvre au survol @F:menu.export-g-code", async ({ page }) => {
    test.setTimeout(90_000);
    await ready(page, []);
    await page.getByRole("button", { name: /^(Fichier|File)$/ }).first().click();
    const exp = page.getByRole("menu").getByRole("button", { name: /^(Exporter|Export)\b/ }).first();
    await exp.hover();
    await expect(exp).toHaveAttribute("aria-expanded", "true", { timeout: 2_000 });
    // Un clic juste après l'ouverture au survol ne le referme pas.
    await exp.click();
    await expect(exp).toHaveAttribute("aria-expanded", "true");
  });

  // ─── B6 : table des paramètres (unités, plage recommandée) ─────────────
  test("table des paramètres : unités séparées des libellés, hauteur de couche trop grande signalée @F:editor.object-settings @F:ui.object-settings", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await ready(page, ["cube20"]);
    await page.getByRole("button", { name: "Objets", exact: true }).click();
    await page.getByTestId("open-object-table").click();
    const table = page.getByTestId("object-table");
    await expect(table.getByTestId("object-table-unit").first()).toHaveText("(mm)");
    const cell = table.getByTestId("object-table-row").first().locator('[data-testid=object-table-cell][data-key="layer_height"]');
    await cell.fill("0.9");
    await cell.press("Enter");
    await expect(table.getByTestId("object-table-error")).toHaveCount(0);
    await expect(table.getByTestId("object-table-warning")).toContainText("trop grande");
  });
});

// ─── E2 : vue réaliste basculée à chaud, couleur du filament conservée ─────
test("vue réaliste (Phong puis Gouraud) activée à chaud : l'objet garde la couleur du filament @F:editor.menu-view-prefs", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Escape");
  const canvas = page.getByTestId("scene-canvas");
  /** Nombre de pixels nettement orangés (filament) dans le canevas. */
  async function colored(): Promise<number> {
    await page.waitForTimeout(1200);
    const b64 = (await canvas.screenshot()).toString("base64");
    return page.evaluate(async (data) => {
      const img = new Image();
      img.src = "data:image/png;base64," + data;
      await img.decode();
      const c = document.createElement("canvas");
      c.width = img.width;
      c.height = img.height;
      const g = c.getContext("2d")!;
      g.drawImage(img, 0, 0);
      const d = g.getImageData(0, 0, c.width, c.height).data;
      let n = 0;
      for (let i = 0; i < d.length; i += 4) if (d[i] > d[i + 1] + 30 && d[i] > d[i + 2] + 30) n++;
      return n;
    }, b64);
  }
  async function graphics(fn: (d: Locator) => Promise<void>) {
    await page.keyboard.press("Control+p");
    const d = page.getByTestId("preferences-dialog");
    await expect(d).toBeVisible();
    await d.getByRole("tab", { name: /Graphismes|Graphics/ }).or(d.getByRole("button", { name: /Graphismes|Graphics/ })).first().click();
    await fn(d);
    await page.keyboard.press("Escape");
    await expect(d).toBeHidden();
  }
  const normal = await colored();
  expect(normal).toBeGreaterThan(100);
  await graphics((d) => d.getByTestId("pref-realistic-view").check());
  await expect(canvas).toHaveAttribute("data-realistic", /phong/);
  expect(await colored()).toBeGreaterThan(normal * 0.5);
  await graphics((d) => d.getByTestId("pref-realistic-phong").uncheck());
  expect(await colored()).toBeGreaterThan(normal * 0.5);
});
