// Phase 35 — branchement des lots 34-35 dans l'interface : ouverture d'un 3MF
// (dialogue projet/géométrie + avertissement G-code modifié), options
// d'arrangement (menu d'Orca + portées + verrouillage de plateau), onglet
// « Projet » (métadonnées, licence, couverture) et administration (désactivation
// de compte, Retry-After du 429).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

async function ready(page: Page, models: string[]) {
  await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
}

// 1×1 PNG (couverture « Model Pictures »).
const PNG_1PX = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGBgAAAABQABh6FO1AAAAABJRU5ErkJggg==",
  "base64",
);

test("Ouverture d'un 3MF : projet ou géométrie seule, avertissement G-code, préférence @F:menu.open-project @F:core.import @F:menu.preferences", async ({ page }) => {
  test.setTimeout(150_000);
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });

  // Un 3MF réel : export du projet courant, réouvert par Fichier › Ouvrir.
  const exported = await page.request.get(`/api/projects/${id}/export/3mf`);
  expect(exported.status()).toBe(200);
  const file3mf = {
    name: "cube.3mf",
    mimeType: "model/3mf",
    buffer: Buffer.from(await exported.body()),
  };

  // Préférence « Toujours demander » (Orca always_ask → ProjectDropDialog).
  await page.keyboard.press("Control+p");
  const dialog = page.getByTestId("preferences-dialog");
  await expect(dialog).toBeVisible();
  await dialog.getByTestId("pref-load-behaviour").selectOption("always_ask");
  await dialog.getByTestId("preferences-close").click();

  await page.getByTestId("project-open-input").setInputFiles(file3mf);
  const ask = page.getByTestId("open3mf-dialog");
  await expect(ask).toBeVisible();
  await expect(page.getByTestId("open3mf-filename")).toContainText("cube.3mf");
  // Défaut d'Orca : « Importer la géométrie seule » présélectionné.
  await expect(page.getByTestId("open3mf-geometry")).toBeChecked();
  await page.getByTestId("open3mf-project").check();
  // « Se souvenir de mon choix » : la boîte ne réapparaîtra plus.
  await page.getByTestId("open3mf-remember").check();
  await page.getByTestId("open3mf-ok").click();
  await expect(page).toHaveURL(/\/projects\//);
  await expect(page.getByTestId("scene-canvas")).toBeVisible({ timeout: 60_000 });
  await expect(ask).toHaveCount(0);

  // Le choix mémorisé s'applique (load_all) : réouverture sans question.
  // (écriture différée du compte : on scrute le serveur jusqu'à la valeur.)
  await expect
    .poll(async () => {
      const prefs = (await (
        await page.request.get("/api/me/preferences")
      ).json()) as Record<string, unknown>;
      return prefs.projectLoadBehaviour;
    })
    .toBe("load_all");
  await page.getByTestId("project-open-input").setInputFiles(file3mf);
  await expect(page.getByTestId("scene-canvas")).toBeVisible({ timeout: 60_000 });
  await expect(ask).toHaveCount(0);

  // Avertissement « G-code modifié » (rapport d'import) : boîte d'Orca avec
  // items + « Ne plus afficher » enregistré dans les préférences du compte.
  const projectId = new URL(page.url()).pathname.split("/").pop()!;
  await page.evaluate((pid) => {
    sessionStorage.setItem(
      `web-slicer.import-report:${pid}`,
      JSON.stringify({
        load: "project",
        gcode_warning: { kind: "modified_gcodes", items: ["machine_start_gcode"] },
        has_metadata: false,
      }),
    );
  }, projectId);
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  const report = page.getByTestId("import-report");
  await expect(report).toBeVisible();
  await expect(report.getByTestId("import-report-items")).toContainText("machine_start_gcode");
  await report.getByTestId("import-report-ignore").check();
  await report.getByTestId("import-report-ok").click();
  await expect(report).toHaveCount(0);
  // « Ne plus afficher » : écrite dans les préférences du compte (différée).
  await expect
    .poll(async () => {
      const after = (await (
        await page.request.get("/api/me/preferences")
      ).json()) as Record<string, unknown>;
      return after.noWarnWhenModifiedGcodes;
    })
    .toBe(true);
});

test("Options d'arrangement (Orca) : espacement, rotation, Reset, portées et verrouillage de plateau @F:toolbar.arrange @F:ctx.arrange @F:editor.plate-settings", async ({ page }) => {
  test.setTimeout(150_000);
  await ready(page, ["cube20", "bridge"]);

  // Le bouton Arrange ouvre le menu d'options (comme le menu ImGui d'Orca).
  await page.getByTestId("toolbar-arrange").click();
  const panel = page.getByTestId("arrange-options-panel");
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId("arrange-multi-materials")).toBeChecked();
  // Rotation auto grise « Aligner sur l'axe Y » (Orca disabled_begin).
  await panel.getByTestId("arrange-rotation").check();
  await expect(panel.getByTestId("arrange-align-y")).toBeDisabled();
  await panel.getByTestId("arrange-rotation").uncheck();
  await expect(panel.getByTestId("arrange-align-y")).toBeEnabled();
  // Espacement au clavier + Réinitialiser (valeurs du serveur).
  await panel.getByTestId("arrange-spacing-input").fill("8");
  await expect(panel.getByTestId("arrange-spacing")).toHaveValue(/8/);
  // Réinitialiser exige les valeurs du serveur (GET …/arrange/options).
  await expect(panel.getByTestId("arrange-reset")).toBeEnabled({ timeout: 30_000 });
  await panel.getByTestId("arrange-reset").click();
  await expect(panel.getByTestId("arrange-spacing")).toHaveValue("0");

  // Verrouillage du plateau courant : « Ce plateau » désactivé (422 évité),
  // puis arrangement de tout le plateau.
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("plate-lock").first().click();
  await expect(page.getByTestId("plate-tab").first()).toHaveAttribute("data-locked", "true");
  await expect(panel.getByTestId("arrange-plate")).toBeDisabled();
  await page.getByTestId("plate-lock").first().click();
  await expect(page.getByTestId("plate-tab").first()).toHaveAttribute("data-locked", "false");
  await expect(panel.getByTestId("arrange-plate")).toBeEnabled();

  await panel.getByTestId("arrange-all").click();
  await expect(page.getByTestId("editor-message")).toContainText(/Arrangement : \d+ objet\(s\)/, {
    timeout: 120_000,
  });
  await expect(page.getByTestId("toolbar-arrange")).toBeVisible();
});

test("Onglet Projet : métadonnées titre/auteur/licence/description et couverture des images jointes @F:ui.auxiliaries", async ({ page }) => {
  test.setTimeout(120_000);
  const id = await bootstrap(page);

  // Une image jointe « Model Pictures » rend la couverture sélectionnable.
  const put = await page.request.put(
    `/api/projects/${id}/auxiliaries/Model%20Pictures/cover.png`,
    { data: PNG_1PX, headers: { "content-type": "image/png" } },
  );
  expect(put.status()).toBe(201);

  await page.getByTestId("config-tab-project").click();
  const meta = page.getByTestId("project-metadata");
  await expect(meta).toBeVisible();
  await meta.getByTestId("project-metadata-title").fill("Mon modèle");
  await meta.getByTestId("project-metadata-designer").fill("Alice");
  await meta.getByTestId("project-metadata-license").selectOption("BY-SA");
  await meta.getByTestId("project-metadata-description").fill("Un cube de test.");
  await meta.getByTestId("project-metadata-cover").selectOption("cover.png");
  await meta.getByTestId("project-metadata-save").click();
  await expect(meta.getByTestId("project-metadata-cover-preview")).toBeVisible();

  // Relues depuis le serveur (GET /metadata) après rechargement.
  await page.reload();
  await expect(page.getByTestId("scene-canvas")).toBeVisible();
  await page.getByTestId("config-tab-project").click();
  await expect(meta.getByTestId("project-metadata-title")).toHaveValue("Mon modèle");
  await expect(meta.getByTestId("project-metadata-license")).toHaveValue("BY-SA");
  await expect(meta.getByTestId("project-metadata-cover")).toHaveValue("cover.png");
});

test("Admin : désactiver / réactiver un compte (PUT status) et Retry-After du 429 @F:core.auth-ui", async ({ page }) => {
  test.setTimeout(120_000);
  await bootstrap(page);
  const me = (await (await page.request.get("/api/auth/me")).json()) as Record<string, string>;
  await page.route("**/api/auth/me", (r) =>
    r.fulfill({ json: { ...me, role: "admin" } }),
  );
  await page.route("**/api/admin/instance", (r) =>
    r.fulfill({
      json: {
        registration_policy: "open",
        upload_limit_bytes: 524288000,
        storage_quota_bytes: 0,
        max_concurrent_slices: 0,
      },
    }),
  );
  const users = [
    {
      id: "u1",
      email: "alice@x.fr",
      role: "admin",
      status: "active",
      created_at: "2026-11-01T10:00:00Z",
      storage_bytes: 0,
      project_count: 2,
    },
    {
      id: "u2",
      email: "bob@x.fr",
      role: "user",
      status: "disabled",
      created_at: "2026-11-02T10:00:00Z",
      storage_bytes: 0,
      project_count: 0,
    },
  ];
  const puts: { url: string; body: string }[] = [];
  await page.route("**/api/admin/users**", (r) => {
    if (r.request().method() === "GET") {
      r.fulfill({ json: { users, total: users.length, page: 1, per_page: 25 } });
    } else if (r.request().method() === "PUT") {
      puts.push({ url: r.request().url(), body: r.request().postData() ?? "" });
      r.fulfill({ status: 204, body: "" });
    } else r.continue();
  });
  await page.goto("/admin");
  const rows = page.getByTestId("admin-user-row");
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(1).getByTestId("admin-user-status")).toHaveText("Désactivé");
  await expect(rows.nth(1).getByTestId("admin-user-enable")).toBeVisible();

  // Désactivation d'un compte actif (PUT /admin/users/{id}/status).
  await rows.first().getByTestId("admin-user-disable").click();
  await expect
    .poll(() => puts.find((p) => p.url.includes("/u1/status"))?.body ?? "")
    .toContain('"disabled"');
  await rows.nth(1).getByTestId("admin-user-enable").click();
  await expect
    .poll(() => puts.find((p) => p.url.includes("/u2/status"))?.body ?? "")
    .toContain('"active"');

  // 429 slice_quota avec Retry-After (lot 35) : secondes affichées.
  await page.unrouteAll();
  const id2 = await bootstrap(page);
  await page.route(`**/api/projects/${id2}/slice`, (r) =>
    r.fulfill({
      status: 429,
      json: {
        code: "slice_quota_exceeded",
        message: "quota",
        details: { active: 2, max: 2, requested: 1, retry_after: 30 },
      },
    }),
  );
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await page.getByTestId("slice-button").click();
  await expect(page.getByText(/— réessayez dans 30 secondes/).first()).toBeVisible({
    timeout: 30_000,
  });
});
