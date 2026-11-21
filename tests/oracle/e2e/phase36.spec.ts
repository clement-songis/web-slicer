// Phase 36 — audit en navigateur des nouveautés 33-35 : correctifs d'interface.
// Arrangement sans rotation qui conserve rotation/échelle (et sans bruit
// flottant), panneau d'arrangement limité à Préparer, fiche du modèle dans la
// page Projet (ProjectPanel d'Orca), avertissement « G-code modifié » avec le
// nom des G-code, libellés visibles du changement de mot de passe, e-mails
// lisibles dans l'administration, carte de position de l'outil sur une ligne.
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { ORACLE, bootstrap, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

const CUBE_CUSTOM = join(
  ORACLE,
  "../../engine/tests/fixtures/custom_gcode/cube_custom.orca.gcode",
);

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

test("Arrangement sans rotation : rotation et échelle conservées ; panneau seulement en Préparer @F:toolbar.arrange @F:gizmo.rotate", async ({ page }) => {
  test.setTimeout(150_000);
  await bootstrap(page);
  await importModel(page, "cube20");
  await importModel(page, "cylinder");
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });

  // Cube tourné de +45° (Page préc.), puis « Tout arranger » (rotation auto off).
  const rows = page.getByTestId("object-row");
  await rows.first().click();
  await page.getByTestId("tool-rotate").click();
  await focusEditor(page);
  await rows.first().click();
  await page.keyboard.press("PageUp");
  const rz = page.getByLabel("Rotation (°) Z", { exact: true });
  await expect.poll(async () => Number(await rz.inputValue())).toBeCloseTo(45, 3);

  await page.getByTestId("toolbar-arrange").click();
  const panel = page.getByTestId("arrange-options-panel");
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId("arrange-rotation")).not.toBeChecked();
  await panel.getByTestId("arrange-all").click();
  await expect(page.getByTestId("editor-message")).toContainText(/Arrangement : \d+ objet\(s\)/, {
    timeout: 120_000,
  });
  await rows.first().click();
  // Valeurs exactes : ni −45 (matrice du serveur), ni « 45.00000000000001 ».
  await expect(rz).toHaveValue("45");
  await expect(page.getByLabel("Rotation (°) X", { exact: true })).toHaveValue("0");
  await expect(page.getByLabel("Échelle X", { exact: true })).toHaveValue("100");

  // Le menu d'arrangement n'existe pas en Aperçu ni dans la page Projet (Orca).
  await page.getByRole("button", { name: "Aperçu", exact: true }).first().click();
  await expect(panel).toHaveCount(0);
  await page.getByRole("button", { name: "Projet", exact: true }).first().click();
  await expect(panel).toHaveCount(0);
  await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
  await expect(panel).toBeVisible();
});

test("Page Projet : fiche du modèle (ProjectPanel d'Orca), une seule fiche montée @F:ui.auxiliaries", async ({ page }) => {
  test.setTimeout(120_000);
  await bootstrap(page);
  await page.getByRole("button", { name: "Projet", exact: true }).first().click();
  const meta = page.getByTestId("project-metadata");
  await expect(meta).toHaveCount(1);
  await meta.getByTestId("project-metadata-title").fill("Support de casque");
  await meta.getByTestId("project-metadata-save").click();
  // L'onglet Projet de la colonne renvoie vers la page (pas de second brouillon).
  await page.getByTestId("config-tab-project").click();
  await expect(page.getByTestId("project-metadata-elsewhere")).toBeVisible();
  await expect(meta).toHaveCount(1);
  // Retour à Préparer : la colonne affiche la fiche enregistrée.
  await page.getByRole("button", { name: "Préparer", exact: true }).first().click();
  await expect(meta.getByTestId("project-metadata-title")).toHaveValue("Support de casque");
});

test("Avertissement G-code modifié : nom des G-code ; préférence sans jargon @F:menu.open-project @F:menu.preferences", async ({ page }) => {
  const id = await bootstrap(page);
  await page.evaluate((pid) => {
    sessionStorage.setItem(
      `web-slicer.import-report:${pid}`,
      JSON.stringify({
        load: "project",
        gcode_warning: { kind: "modified_gcodes", items: ["machine_start_gcode", "inconnu_gcode"] },
        has_metadata: false,
      }),
    );
  }, id);
  await page.reload();
  const items = page.getByTestId("import-report-items");
  await expect(items).toBeVisible();
  await expect(items.locator('[data-key="machine_start_gcode"]')).toContainText(
    /G-code de démarrage de la machine|Machine start G-code/,
  );
  await expect(items.locator('[data-key="machine_start_gcode"]')).toContainText("(machine_start_gcode)");
  await expect(items.locator('[data-key="inconnu_gcode"]')).toHaveText("inconnu_gcode");
  await page.getByTestId("import-report-ok").click();

  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const dialog = page.getByTestId("preferences-dialog");
  await dialog.getByRole("tab", { name: "Comportement" }).click();
  await expect(dialog).toContainText("Avertir du G-code modifié");
  await expect(dialog).not.toContainText(/lot \d+/);
});

test("Compte : libellés visibles du changement de mot de passe ; admin : e-mails lisibles @F:core.auth-ui", async ({ page }) => {
  await bootstrap(page);
  await page.goto("/library");
  await page.getByLabel("Menu utilisateur").click();
  await page.getByTestId("account-password-open").click();
  const form = page.getByTestId("account-password-form");
  await expect(form.getByText("Mot de passe actuel", { exact: true })).toBeVisible();
  await expect(form.getByText("(8 caractères min.)")).toBeVisible();
  await expect(form.locator("input[placeholder]")).toHaveCount(0);
  await expect(form.getByLabel("Nouveau mot de passe", { exact: true })).toBeVisible();

  const me = (await (await page.request.get("/api/auth/me")).json()) as Record<string, string>;
  await page.route("**/api/auth/me", (r) => r.fulfill({ json: { ...me, role: "admin" } }));
  await page.route("**/api/admin/instance", (r) =>
    r.fulfill({
      json: { registration_policy: "open", upload_limit_bytes: 524288000, storage_quota_bytes: 0, max_concurrent_slices: 0 },
    }),
  );
  const email = "beatrice.dupont-lefebvre@atelier-impression.fr";
  await page.route("**/api/admin/users**", (r) =>
    r.fulfill({
      json: {
        users: [
          { id: "u9", email, role: "user", status: "active", created_at: "2026-11-02T10:00:00Z", storage_bytes: 0, project_count: 3 },
        ],
        total: 1,
        page: 1,
        per_page: 25,
      },
    }),
  );
  await page.goto("/admin");
  const row = page.getByTestId("admin-user-row");
  await expect(row).toHaveCount(1);
  const cell = row.getByTitle(email);
  const box = await cell.boundingBox();
  expect(box!.width).toBeGreaterThanOrEqual(220);
  await expect(row.getByTestId("admin-user-disable")).toBeVisible();
});

test("G-code externe : position de l'outil sur une seule ligne @F:editor.menu-open-gcode @F:preview.engine", async ({ page }) => {
  test.setTimeout(150_000);
  await bootstrap(page);
  await page.getByTestId("gcode-open-input").setInputFiles(CUBE_CUSTOM);
  await expect(page.getByTestId("engine-summary")).toHaveAttribute("data-source", "engine", { timeout: 60_000 });
  const card = page.getByTestId("preview-tool-position");
  await expect(card).toBeVisible();
  // Une ligne (≈ 30 px) : coordonnées, vitesse et temps ne passent pas à la ligne.
  await expect.poll(async () => (await card.boundingBox())!.height).toBeLessThan(40);
  await expect(page.getByTestId("preview-tool-time")).toBeVisible();
});
