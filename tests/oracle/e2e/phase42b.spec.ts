// Phase 42b — API du lot 42 branchées dans l'interface : calibration nommée par
// le serveur (K1), nom de fichier choisi à l'envoi (M2), file d'envoi /
// historique (M3), refus d'adresse d'hôte traduits et 409 « non connectée »
// (M1), signature de tranchage du serveur à la réouverture depuis un autre
// navigateur (V1).
//
// Oracle Orca 2.4.1 : Plater (projet de calibration = nom du modèle chargé),
// PrintHostSendDialog (nom pré-rempli, modifiable, extension imposée),
// PrintHostQueueDialog (« Print host upload queue » : état, hôte, taille,
// fichier, message), OctoPrint::make_url + Http::get_host_from_url (adresse),
// Plater::priv::load_files (aucun résultat périmé rouvert).
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel, openParam, sliceInUi } from "../lib/app";

const MOCK = process.env.MOONRAKER_URL ?? "http://127.0.0.1:17125";
const MOCK_HOST = MOCK.replace(/^https?:\/\//, "");

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
/** Ouvre la boîte d'envoi et connecte l'imprimante au Moonraker simulé. */
async function openSendConnected(page: Page) {
  await page.getByTestId("send-to-printer").click();
  const dialog = page.getByTestId("send-dialog");
  await expect(dialog).toBeVisible();
  const url = page.getByTestId("send-moonraker-url");
  // Liste des imprimantes chargée : champ d'adresse ou hôte déjà connecté.
  await expect(url.or(page.getByTestId("send-host-info"))).toBeVisible();
  if (await url.isVisible()) {
    await url.fill(MOCK_HOST);
    await page.getByTestId("send-save-url").click();
  }
  await expect(page.getByTestId("send-host-info")).toBeVisible();
  return dialog;
}

test.describe("lot 42 dans l'interface (FR)", () => {
  test.use({ locale: "fr-FR" });

  // ─── K1 : calibration nommée par le serveur ─────────────────────────────
  test("K1 : calibration relancée → nom lisible et unique posé par le serveur, sans renommage client @F:editor.calibration-retraction", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    const sourceId = await bootstrap(page);
    const renames: string[] = [];
    page.on("request", (r) => {
      if (r.method() === "PATCH" || (r.method() === "PUT" && /\/api\/projects\/[^/]+\/name$/.test(r.url())))
        renames.push(r.url());
    });
    const launch = async () => {
      await page.getByRole("button", { name: "Calibration", exact: true }).click();
      await page.getByRole("menuitem", { name: "Test de rétraction" }).click();
      const dialog = page.getByTestId("engine-calib-dialog");
      await expect(dialog).toBeVisible();
      const [res] = await Promise.all([
        page.waitForResponse((r) => r.request().method() === "POST" && /\/calibration$/.test(r.url()), {
          timeout: 120_000,
        }),
        dialog.getByTestId("engine-calib-confirm").click(),
      ]);
      expect(res.status()).toBeLessThan(300);
      // La page navigue aussitôt vers le nouveau projet : nom relu par l'API.
      const id = res.url().split("/projects/")[1].split("/")[0];
      await expect(page).toHaveURL(new RegExp(id), { timeout: 180_000 });
      await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 180_000 });
      return (await (await page.request.get("/api/projects/" + id)).json()) as { name: string };
    };
    const first = await launch();
    expect(page.url()).not.toContain(sourceId);
    expect(first.name).not.toMatch(/_|^Calibration( \(\d+\))?$/);
    await expect(page).toHaveTitle(new RegExp("^" + first.name.replace(/[()]/g, "\\$&")));
    const second = await launch();
    expect(second.name).toBe(first.name + " (2)");
    await expect(page.getByTestId("project-name")).toHaveText(second.name);
    // Projets enregistrés : noms du serveur, aucun libellé provisoire restant.
    const projects = (await (await page.request.get("/api/projects")).json()) as { name: string }[];
    const names = projects.map((p) => p.name);
    expect(names).toContain(first.name);
    expect(names).toContain(second.name);
    expect(names.some((n) => /^Calibration( \(\d+\))?$|Calibration retraction/.test(n))).toBe(false);
    expect(renames).toEqual([]);
  });

  // ─── M2 / M3 : nom de fichier choisi, file d'envoi ──────────────────────
  test("M2/M3 : nom pré-rempli et validé en direct, envoi avec ce nom, file d'envoi (dialogue, Appareil, Imprimantes) @F:core.moonraker", async ({
    page,
  }) => {
    test.setTimeout(300_000);
    await ready(page, ["cube20"]);
    await sliceInUi(page);
    const dialog = await openSendConnected(page);
    const name = dialog.getByTestId("send-filename");
    await expect(name).not.toHaveValue("", { timeout: 15_000 });
    await expect(dialog.getByTestId("send-filename-ext")).toHaveText(".gcode");
    expect(await name.inputValue()).not.toMatch(/\.gcode$/);
    const confirm = dialog.getByTestId("send-confirm");
    // Raisons traduites (contrôle immédiat, mêmes règles que le serveur).
    await name.fill("a/b");
    await expect(dialog.getByTestId("send-filename-error")).toHaveAttribute("data-reason", "chars");
    await expect(dialog.getByTestId("send-filename-error")).toContainText("caractères interdits");
    await expect(confirm).toBeDisabled();
    await name.fill(".cache");
    await expect(dialog.getByTestId("send-filename-error")).toContainText("ne peut pas commencer par un point");
    await name.fill("   ");
    await expect(dialog.getByTestId("send-filename-error")).toContainText("Saisissez un nom de fichier");
    await name.fill("x".repeat(220));
    await expect(dialog.getByTestId("send-filename-error")).toContainText("trop long");
    // Nom valable : vérifié par le serveur (POST …/upload-name avec filename).
    const check = page.waitForRequest(
      (r) => /\/upload-name$/.test(r.url()) && (r.postData() ?? "").includes("Lot42b essai"),
    );
    await name.fill("Lot42b essai");
    await check;
    await expect(dialog.getByTestId("send-filename-error")).toHaveCount(0);
    const [upload] = await Promise.all([
      page.waitForRequest((r) => r.method() === "POST" && /\/api\/printers\/[^/]+\/upload$/.test(r.url())),
      confirm.click(),
    ]);
    expect(JSON.parse(upload.postData() ?? "{}").filename).toBe("Lot42b essai.gcode");
    await expect(dialog.getByTestId("send-success")).toContainText("Lot42b essai.gcode", { timeout: 60_000 });
    // File d'envoi ouverte après l'envoi : ligne en évidence, état Terminé.
    await dialog.getByTestId("send-show-queue").click();
    const queue = page.getByTestId("upload-queue");
    await expect(queue).toBeVisible();
    const row = queue.locator('[data-testid=upload-row][data-highlight="true"]');
    await expect(row).toHaveCount(1);
    await expect(row.getByTestId("upload-filename")).toHaveText("Lot42b essai.gcode");
    await expect(row).toHaveAttribute("data-status", "completed", { timeout: 15_000 });
    await expect(row.getByTestId("upload-status")).toContainText("Terminé");
    await expect(row).toContainText(MOCK_HOST);
    await page.keyboard.press("Escape");
    await expect(queue).toHaveCount(0);
    await page.keyboard.press("Escape");
    // Onglet Appareil.
    await page.getByRole("button", { name: /^(appareil|device)$/i }).first().click();
    await page.getByTestId("device-upload-queue").click();
    await expect(page.getByTestId("upload-queue").getByTestId("upload-filename")).toHaveText(["Lot42b essai.gcode"]);
    await page.keyboard.press("Escape");
    // Page Imprimantes : vue de toutes les imprimantes, puis vidage.
    await page.goto("/printers");
    await page.getByTestId("printers-upload-queue").click();
    const all = page.getByTestId("upload-queue");
    await expect(all.getByTestId("upload-row")).toHaveCount(1);
    await expect(all.locator("thead")).toContainText("Imprimante");
    await all.getByTestId("upload-queue-clear").click();
    await expect(all.getByTestId("upload-queue-clear")).toContainText("Vider l'historique de toutes les imprimantes ?");
    await all.getByTestId("upload-queue-clear").click();
    await expect(all.getByTestId("upload-queue-empty")).toBeVisible();
  });

  test("M2 : plusieurs plateaux — base commune envoyée à upload-sliced @F:core.moonraker @F:toolbar.addplate", async ({
    page,
  }) => {
    test.setTimeout(420_000);
    await ready(page, ["cube20"]);
    await page.getByTestId("toolbar-addplate").click();
    await importModel(page, "cylinder");
    await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
    await sliceInUi(page);
    const dialog = await openSendConnected(page);
    await dialog.getByTestId("send-scope-all").click();
    await expect(dialog.getByTestId("send-filename-hint")).toContainText("Nom de base commun");
    await dialog.getByTestId("send-filename").fill("Lot42b multi");
    const [req] = await Promise.all([
      page.waitForRequest((r) => /\/upload-sliced$/.test(r.url()), { timeout: 300_000 }),
      dialog.getByTestId("send-confirm").click(),
    ]);
    expect(JSON.parse(req.postData() ?? "{}").filename).toBe("Lot42b multi.gcode");
    await expect(dialog.getByTestId("send-success")).toContainText("Lot42b multi", { timeout: 120_000 });
    const res = await (await req.response())!.json();
    const files = (res.files as { filename?: string; path: string }[]).map((f) => f.filename ?? f.path);
    expect(files).toEqual(["Lot42b multi.gcode", "Lot42b multi_2.gcode"]);
  });

  // ─── M1 : refus serveur traduits, 409 → adresse redemandée ─────────────
  test("M1 : refus de l'adresse par le serveur traduit (query) ; 409 printer_not_connected → saisie de l'adresse @F:core.moonraker", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20"]);
    await sliceInUi(page);
    await page.getByTestId("send-to-printer").click();
    const url = page.getByTestId("send-moonraker-url");
    await url.fill(MOCK_HOST + "/?cle=1");
    const [put] = await Promise.all([
      page.waitForResponse((r) => r.request().method() === "PUT" && /\/api\/printers\/[^/]+$/.test(r.url())),
      page.getByTestId("send-save-url").click(),
    ]);
    expect(put.status()).toBe(422);
    expect((await put.json()).details.reason).toBe("query");
    await expect(page.getByTestId("send-url-error")).toContainText("retirez la partie « ? » ou « # »");
    await expect(page.getByTestId("send-url-error")).toContainText("Exemples");
    await url.fill(MOCK_HOST);
    await page.getByTestId("send-save-url").click();
    await expect(page.getByTestId("send-host-info")).toBeVisible();
    // Adresse enregistrée devenue inutilisable (antérieure au lot 42) : 409.
    await page.route(/\/api\/printers\/[^/]+\/upload$/, (route) =>
      route.fulfill({
        status: 409,
        contentType: "application/json",
        body: JSON.stringify({
          code: "printer_not_connected",
          message: "Adresse de l'hôte invalide",
          details: { field: "moonraker_url", reason: "whitespace", value: "pas une url" },
        }),
      }),
    );
    await page.getByTestId("send-confirm").click();
    await expect(url).toBeVisible();
    await expect(page.getByTestId("send-url-error")).toContainText("une adresse ne contient pas d'espace");
    await expect(page.getByTestId("send-error")).toHaveCount(0);
    // Page Imprimantes : même traduction dans le dialogue de connexion.
    await page.unroute(/\/api\/printers\/[^/]+\/upload$/);
    await page.goto("/printers");
    await page.getByTestId("printer-connection").first().click();
    const conn = page.getByTestId("printer-connection-dialog");
    await expect(conn).toBeVisible();
    await conn.getByTestId("printer-moonraker-url").fill(MOCK + "/?x=1");
    await conn.getByTestId("connection-save").click();
    await expect(conn.getByTestId("connection-error")).toContainText("retirez la partie « ? » ou « # »");
    await expect(conn.getByTestId("connection-error")).toContainText("Exemples");
  });

  // ─── V1 : signature serveur depuis un autre navigateur ─────────────────
  test("V1 : autre navigateur — résultat restauré si la signature serveur est fraîche, sinon Préparer @F:core.slice-ui @F:core.project-save", async ({
    page,
    browser,
  }) => {
    test.setTimeout(360_000);
    const projectId = await ready(page, ["cube20"]);
    await saveProject(page);
    await sliceInUi(page);
    const state = await page.context().storageState();
    const other = async () => {
      const ctx = await browser.newContext({ storageState: { cookies: state.cookies, origins: [] }, locale: "fr-FR" });
      const p = await ctx.newPage();
      const sig = p.waitForResponse((r) => r.url().endsWith("/projects/" + projectId + "/slice-signature"), {
        timeout: 90_000,
      });
      await p.goto("/projects/" + projectId);
      const res = await sig;
      expect(res.status()).toBe(200);
      return { ctx, p, body: (await res.json()) as { plates: { fresh: boolean; last_gcode_id?: string }[] } };
    };
    // Autre navigateur (aucune empreinte locale) : verdict serveur « fresh ».
    const b = await other();
    expect(b.body.plates[0].fresh).toBe(true);
    await expect(b.p.getByTestId("preview-view")).toBeVisible({ timeout: 60_000 });
    await expect(b.p.getByTestId("export-gcode-button")).toBeEnabled({ timeout: 30_000 });
    // Réglage modifié et enregistré dans ce 2e navigateur.
    await b.p.getByRole("button", { name: "Préparer", exact: true }).first().click();
    const control = await openParam(b.p, "print", "Strength", "wall_loops");
    await expect(b.p.getByTestId("settings-dialog").locator('fieldset[aria-busy="true"]')).toHaveCount(0);
    await expect(control).not.toHaveValue("");
    await control.fill(String(Number(await control.inputValue()) + 2));
    await control.press("Tab");
    await b.p.keyboard.press("Escape");
    await saveProject(b.p);
    await b.ctx.close();
    // 3e navigateur : signature périmée → Préparer, export et envoi désactivés.
    const c = await other();
    expect(c.body.plates[0].fresh).toBe(false);
    await expect(c.p.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
    await c.p.waitForTimeout(4_000);
    await expect(c.p.getByTestId("preview-view")).toHaveCount(0);
    await expect(c.p.getByTestId("export-gcode-button")).toBeDisabled();
    await expect(c.p.getByTestId("send-to-printer")).toBeDisabled();
    await c.ctx.close();
  });
});
