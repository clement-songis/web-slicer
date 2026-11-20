// Phase 33 — lot 31 branché dans l'aperçu : G-code externe simulé par le moteur
// (POST /api/gcode/process + /gcode/opened/{token}/meta|moves|seams), vue
// « Couleur » (ColorPrint d'Orca : palette, couches à pause grisées, repères du
// curseur), temps estimé à la position de l'outil (get_estimated_time_at),
// presets actifs envoyés à /slice (active_presets), tableau « Filament » par
// usage et statistiques de tous les plateaux (POST /api/gcodes/plates-stats).
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { ORACLE, bootstrap, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

// G-code de référence d'Orca 2.4.1 : G-code perso / pause / perso aux cotes 5, 10, 15.
const CUBE_CUSTOM = join(
  ORACLE,
  "../../engine/tests/fixtures/custom_gcode/cube_custom.orca.gcode",
);

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

async function goToLayer(page: Page, n: number) {
  await focusEditor(page);
  await page.keyboard.press("Shift+G");
  const form = page.getByTestId("preview-goto-layer");
  await form.getByRole("spinbutton").fill(String(n));
  await form.getByRole("spinbutton").press("Enter");
  await expect(page.getByTestId("preview-canvas")).toHaveAttribute(
    "data-layer-high",
    String(n - 1),
  );
}

test("G-code externe simulé : stats moteur, vue Couleur, repères, temps à l’outil, coutures @F:editor.menu-open-gcode @F:preview.engine", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await bootstrap(page);
  const calls: string[] = [];
  page.on("request", (r) => {
    const path = new URL(r.url()).pathname;
    if (path.startsWith("/api/gcode/"))
      calls.push(`${r.method()} ${path.replace(/opened\/[^/]+/, "opened/T")}`);
  });
  await page.getByTestId("gcode-open-input").setInputFiles(CUBE_CUSTOM);
  await expect(page.getByTestId("editor-message")).toContainText(
    "cube_custom.orca.gcode",
    { timeout: 60_000 },
  );
  // Résumé = simulation GCodeProcessor (= en-tête d'Orca 9m 55s), pas le pied relu.
  const summary = page.getByTestId("engine-summary");
  await expect(summary).toHaveAttribute("data-source", "engine", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("engine-total-time")).toHaveText("9m 55s");
  expect(calls).toContain("POST /api/gcode/process");
  await expect
    .poll(() =>
      calls.some((c) => c.startsWith("GET /api/gcode/opened/T/seams")),
    )
    .toBe(true);

  // Repères du curseur : perso (cote 5), pause (10), perso (15).
  const ticks = page.getByTestId("layer-tick");
  await expect(ticks).toHaveCount(3);
  await expect(ticks.nth(0)).toHaveAttribute("data-type", "custom");
  await expect(ticks.nth(1)).toHaveAttribute("data-type", "pause_print");

  // Vue « Couleur » : palette de la visionneuse (#FF8000), couches à option grisées.
  await page.getByLabel("Coloration de l'aperçu").selectOption("color-print");
  const canvas = page.getByTestId("preview-canvas");
  await expect(canvas).toHaveAttribute("data-coloration", "color-print");
  const legend = page.getByTestId("legend-entry");
  await expect(legend.filter({ hasText: "Filament 1" })).toHaveCount(1);
  await expect(
    legend.filter({ hasText: "Pause / G-code personnalisé" }),
  ).toHaveCount(1);
  await goToLayer(page, 10);
  await expect(canvas).toHaveAttribute("data-colors", "#ff8000");
  await goToLayer(page, 60);
  await expect(canvas).toHaveAttribute("data-colors", /#404040/);
  await expect(canvas).toHaveAttribute("data-colors", /#ff8000/);

  // Temps estimé à la position de l'outil : fin de la dernière couche = total.
  await goToLayer(page, 100);
  await focusEditor(page);
  await page.keyboard.press("End");
  const time = page.getByTestId("preview-tool-time");
  await expect(time).toBeVisible({ timeout: 30_000 });
  await expect
    .poll(async () => Number(await time.getAttribute("data-time")))
    .toBeGreaterThan(594.8);
  expect(Number(await time.getAttribute("data-time"))).toBeLessThan(595.8);
  await expect(page.getByTestId("preview-move-time")).toHaveText("9m 55s");
  // Curseur déplacé : temps exact au mouvement (WSPm de la couche chargé),
  // avec le temps du mouvement seul entre parenthèses.
  await page.keyboard.press("ArrowLeft");
  await expect
    .poll(async () => Number(await time.getAttribute("data-time")))
    .toBeLessThan(595.3);
  expect(Number(await time.getAttribute("data-time"))).toBeGreaterThan(585);
  await expect(time).toContainText(/\(\d+,\d{3} s\)/);
  // Départ de couche : temps écoulé avant la couche, plus petit.
  await page.keyboard.press("Home");
  await expect
    .poll(async () => Number(await time.getAttribute("data-time")))
    .toBeLessThan(594);
  await page.getByTestId("preview-tool-toggle").click();
  await expect(page.getByTestId("preview-tool-properties")).toContainText(
    "Temps estimé",
  );
});

test("presets actifs envoyés à /slice ; tableau Filament par usage ; vue Couleur multi-filament @F:editor.filament-slots @F:core.slice-ui", async ({
  page,
}) => {
  test.setTimeout(420_000);
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByTestId("add-filament-slot").click();
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByLabel("Extrudeur de cylinder.stl").selectOption("2");
  await page.getByRole("button", { name: /^(Traitement|Process)$/ }).click();
  const bodies: Record<string, unknown>[] = [];
  page.on("request", (req) => {
    if (
      req.url().endsWith(`/api/projects/${id}/slice`) &&
      req.method() === "POST"
    )
      bodies.push(req.postDataJSON() as Record<string, unknown>);
  });
  await page.getByTestId("slice-button").click();
  await expect(page.getByTestId("preview-view")).toHaveAttribute(
    "data-layer-count",
    /^[1-9]\d*$/,
    { timeout: 300_000 },
  );
  // Presets affichés (2 filaments) transmis avec la scène : le serveur tranche
  // la configuration éditée sans dépendre d'un enregistrement préalable.
  expect(bodies).toHaveLength(1);
  if (bodies[0].scene !== undefined) {
    const presets = bodies[0].active_presets as { filaments?: string[] };
    expect(presets?.filaments).toHaveLength(2);
  }
  // Tableau Filament (modèle / purge / tour) par filament.
  await expect(page.getByTestId("engine-summary")).toHaveAttribute(
    "data-source",
    "engine",
    { timeout: 60_000 },
  );
  const table = page.getByTestId("engine-filament-table");
  await expect(table).toBeVisible();
  await expect(table.locator("th[data-column=tower]")).toHaveCount(1);
  await expect(table.locator("tbody tr")).toHaveCount(2);
  // Vue Couleur : une entrée par filament utilisé.
  await page.getByLabel("Coloration de l'aperçu").selectOption("color-print");
  const legend = page.getByTestId("legend-entry");
  await expect(legend).toHaveCount(2);
  await expect(legend.nth(1)).toContainText("Filament 2");
});

test("statistiques de tous les plateaux : tableau Filament cumulé (plates-stats) @F:editor.preview-plates", async ({
  page,
}) => {
  test.setTimeout(420_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await page
    .getByLabel("Configuration")
    .getByRole("button", { name: "Objets", exact: true })
    .click();
  await page
    .getByTestId("plate-tab")
    .first()
    .locator("xpath=../..")
    .getByRole("button", { name: "Ajouter un plateau", exact: true })
    .click();
  await page
    .getByTestId("plate-tab")
    .last()
    .getByRole("button")
    .first()
    .click();
  await importModel(page, "cylinder");
  const stats: string[][] = [];
  page.on("request", (r) => {
    if (r.method() === "POST" && r.url().endsWith("/api/gcodes/plates-stats"))
      stats.push((r.postDataJSON() as { gcode_ids: string[] }).gcode_ids);
  });
  await page.getByTestId("slice-menu-button").click();
  await page.getByTestId("slice-all").click();
  await expect(page.getByTestId("preview-view")).toHaveAttribute(
    "data-layer-count",
    /^[1-9]\d*$/,
    { timeout: 300_000 },
  );
  const all = page.getByTestId("preview-plates-all");
  await expect(all).toBeVisible({ timeout: 240_000 });
  await expect(page.getByTestId("preview-plates-totals")).toContainText(
    "Tous les plateaux (2)",
  );
  expect(stats.at(-1)).toHaveLength(2);
  await page.getByTestId("preview-plates-detail").click();
  const table = page.getByTestId("preview-plates-filaments");
  await expect(table).toBeVisible();
  await expect(table.locator("th[data-column=model]")).toHaveCount(1);
  await expect(table.locator("tbody tr")).toHaveCount(1);
});

// ── Lot 32 (comptes, quotas) ────────────────────────────────────────────────
// Routes servies seulement après redéploiement du backend : le test sur le
// vrai serveur est sauté tant que GET /api/me/usage répond 404.

test("compte : jauge d’usage, changer son mot de passe (serveur lot 32) @F:core.auth-ui", async ({
  page,
}) => {
  await bootstrap(page);
  const probe = await page.request.get("/api/me/usage");
  test.skip(
    probe.status() === 404,
    "backend antérieur au lot 32 (route absente)",
  );
  await page.goto("/library");
  await page.getByLabel("Menu utilisateur").click();
  await expect(page.getByTestId("account-usage-storage")).toBeVisible();
  await page.getByTestId("account-password-open").click();
  const form = page.getByTestId("account-password-form");
  await form.getByLabel("Mot de passe actuel").fill("mauvais-mot-de-passe");
  await form
    .getByLabel("Nouveau mot de passe", { exact: true })
    .fill("nouveau-mdp-123");
  await form
    .getByLabel("Confirmer le nouveau mot de passe")
    .fill("nouveau-mdp-123");
  await form.getByRole("button", { name: "Changer" }).click();
  await expect(page.getByTestId("account-password-message")).toHaveText(
    "Mot de passe actuel incorrect.",
  );
  await form.getByLabel("Mot de passe actuel").fill("harness-password-123");
  await form.getByRole("button", { name: "Changer" }).click();
  await expect(page.getByTestId("account-password-message")).toContainText(
    "Mot de passe changé",
  );
  // La session courante reste ouverte.
  expect((await page.request.get("/api/auth/me")).status()).toBe(200);
});

test("interface lot 32 (réponses simulées) : liste des comptes, quotas, 413 et 429 explicites @F:core.auth-ui", async ({
  page,
}) => {
  await bootstrap(page);
  const me = (await (await page.request.get("/api/auth/me")).json()) as Record<
    string,
    string
  >;
  await page.route("**/api/auth/me", (r) =>
    r.fulfill({ json: { ...me, role: "admin" } }),
  );
  await page.route("**/api/admin/instance", (r) =>
    r.fulfill({
      json: {
        registration_policy: "open",
        upload_limit_bytes: 524288000,
        storage_quota_bytes: 1048576000,
        max_concurrent_slices: 2,
      },
    }),
  );
  const queries: string[] = [];
  await page.route("**/api/admin/users?**", (r) => {
    const q = new URL(r.request().url()).searchParams.get("q") ?? "";
    queries.push(q);
    const users = [
      {
        id: "u1",
        email: "alice@x.fr",
        role: "admin",
        status: "active",
        created_at: "2026-11-01T10:00:00Z",
        storage_bytes: 5242880,
        project_count: 3,
      },
      {
        id: "u2",
        email: "bob@x.fr",
        role: "user",
        status: "active",
        created_at: "2026-11-02T10:00:00Z",
        storage_bytes: 0,
        project_count: 1,
      },
    ].filter((u) => u.email.includes(q));
    r.fulfill({ json: { users, total: users.length, page: 1, per_page: 25 } });
  });
  await page.route("**/api/me/usage", (r) =>
    r.fulfill({
      json: {
        storage_bytes: 943718400,
        storage_quota_bytes: 1048576000,
        active_slices: 2,
        max_concurrent_slices: 2,
      },
    }),
  );
  await page.goto("/admin");
  await expect(page.getByTestId("admin-storage-quota")).toHaveValue("1000");
  await expect(page.getByTestId("admin-slice-quota")).toHaveValue("2");
  const rows = page.getByTestId("admin-user-row");
  await expect(rows).toHaveCount(2);
  await expect(rows.first()).toContainText("3 projets · 5 Mo");
  await page.getByTestId("admin-user-search").fill("bob");
  await expect(rows).toHaveCount(1);
  expect(queries.at(-1)).toBe("bob");
  await expect(page.getByTestId("admin-user-count")).toHaveText("1–1 sur 1");
  // Jauge du menu compte (90 %).
  await page.getByLabel("Menu utilisateur").click();
  await expect(page.getByTestId("account-usage-storage")).toHaveText(
    "900 Mo sur 1 000 Mo",
  );
  await expect(page.getByTestId("account-usage-slices")).toHaveText(
    "2 sur 2 en cours",
  );
  await expect(
    page.getByRole("meter", { name: "Stockage utilisé" }),
  ).toHaveAttribute("aria-valuenow", "90");

  // 413 / 429 : messages explicites dans l'éditeur.
  await page.unrouteAll();
  const id = await bootstrap(page);
  await page.route(`**/api/projects/${id}/slice`, (r) =>
    r.fulfill({
      status: 429,
      json: {
        code: "slice_quota_exceeded",
        message: "quota",
        details: { active: 2, max: 2, requested: 1 },
      },
    }),
  );
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByTestId("slice-button").click();
  await expect(
    page.getByText(/Trop de tranchages en cours \(2 sur 2 autorisés\)/).first(),
  ).toBeVisible({
    timeout: 30_000,
  });
  await page.route(`**/api/projects/${id}/models`, (r) =>
    r.request().method() === "POST"
      ? r.fulfill({
          status: 413,
          json: {
            code: "storage_quota_exceeded",
            message: "quota",
            details: {
              used_bytes: 1047527424,
              quota_bytes: 1048576000,
              incoming_bytes: 2097152,
            },
          },
        })
      : r.fallback(),
  );
  await page
    .getByTestId("import-input")
    .setInputFiles(join(ORACLE, "fixtures/cylinder.stl"));
  await expect(
    page
      .getByText(/Espace de stockage plein : 999 Mo utilisés sur 1\s000 Mo/)
      .first(),
  ).toBeVisible({
    timeout: 30_000,
  });
});
