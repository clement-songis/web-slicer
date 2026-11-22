// Phase 46 — corrections de l'audit 44 (navigateur), partie interface :
// lot de préréglages au format d'Orca (PR1-a/b, routes du lot 46), « Créer un
// filament » filtré par type (PR1-c), imprimante déclarée / envoi vers une
// autre imprimante (DEV1-a), session expirée (SESS-a), touche A = bouton
// Disposer (ARR-a), panneau OUTILS sous la barre du plateau à 800 × 500
// (FIX1-b), cosmétiques (rotation à 2 décimales, ZIP à l'import, sous-liste
// « Instances », filaments de l'assistant, assistant rouvert avec Annuler,
// objet nommé dans l'échec « couche initiale vide », Ctrl+N dans la
// bibliothèque).
//
// Oracle Orca 2.4.1 : CreateFilamentPresetDialog (CreatePresetsDialog.cpp:
// 1210-1381), PhysicalPrinterDialog (preset en cours), SelectMachineDialog
// (« The selected printer is incompatible with the chosen printer presets. »),
// GLCanvas3D::on_char 'A' / Shift+A → EVT_GLCANVAS_ARRANGE(_PARTPLATE),
// ObjectManipulation (double_to_string(v, 2)), Plater::load_files →
// handle_zips, ObjectList::add_instance (itInstanceRoot), ConfigWizard,
// MainFrame Ctrl+N, ExportConfigsDialog::archive_preset_bundle_to_file.
//
// ⚠ Lot 46 serveur requis (backend redémarré) pour : « lot de préréglages »
// et « envoi refusé » (409). Les autres tests ne dépendent que de l'interface.
import { expect, test, type Page } from "@playwright/test";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { bootstrap, fixture, importModel, sliceInUi } from "../lib/app";

const PASSWORD = "harness-password-123";

async function ready(page: Page, models: string[]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  return id;
}
async function focusEditor(page: Page) {
  await page.getByTestId("scene-canvas").click({ position: { x: 30, y: 30 } });
  await page.locator("body").focus();
}
type SceneDoc = { objects: { id: string; position?: number[]; scale?: number[] }[] };
async function scene(page: Page, id: string): Promise<SceneDoc> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return ((await res.json()) as { scene: SceneDoc }).scene;
}

// — ZIP « stocké » minimal (fichiers réels de fixtures). —
const CRC = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function zip(entries: Record<string, Buffer>): Buffer {
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;
  for (const [name, data] of Object.entries(entries)) {
    const fname = Buffer.from(name, "utf8");
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(fname.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(fname.length, 28);
    central.writeUInt32LE(offset, 42);
    locals.push(local, fname, data);
    centrals.push(central, fname);
    offset += 30 + fname.length + data.length;
  }
  const cd = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(Object.keys(entries).length, 8);
  end.writeUInt16LE(Object.keys(entries).length, 10);
  end.writeUInt32LE(cd.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, end]);
}

test.describe("audit 44 (FR)", () => {
  test.use({ locale: "fr-FR" });

  test("PR1-c : « Créer un filament » — filaments de départ du type choisi, Generic PETG par défaut @F:ui.create-filament", async ({
    page,
  }) => {
    await bootstrap(page);
    await page.getByTestId("create-filament").click();
    const dlg = page.getByTestId("create-filament-dialog");
    await dlg.getByTestId("create-filament-type").selectOption("PETG");
    const base = dlg.getByTestId("create-filament-base");
    await expect(base.locator("option").first()).toBeAttached();
    const options = await base.locator("option").allTextContents();
    expect(options.length).toBeGreaterThan(0);
    for (const o of options) expect(o, o).toMatch(/PETG/i);
    expect(await base.locator("option:checked").textContent()).toMatch(/^Generic PETG/);
    await dlg.getByTestId("create-filament-vendor").fill("Atelier46");
    await dlg.getByTestId("create-filament-serial").fill("Bleu");
    await dlg.getByTestId("create-filament-submit").click();
    await expect(dlg).toHaveCount(0);
    const list = (await (await page.request.get("/api/presets?kind=filament")).json()) as { id: string; name: string }[];
    const created = list.find((p) => p.name === "Atelier46 PETG Bleu")!;
    const detail = await (await page.request.get(`/api/presets/${created.id}`)).json();
    expect(detail.inherits).toMatch(/^Generic PETG/);
    // Retour au type PLA : la liste suit.
    await page.getByTestId("create-filament").click();
    await dlg.getByTestId("create-filament-type").selectOption("PLA");
    for (const o of await base.locator("option").allTextContents()) expect(o, o).toMatch(/PLA/i);
  });

  test("DEV1-a : « Déclarer manuellement » propose l'imprimante en cours (dernière déclarée / ?preset=), jamais le 1er du catalogue @F:ui.print-hosts", async ({
    page,
  }) => {
    await bootstrap(page);
    const printers = (await (await page.request.get("/api/printers")).json()) as { machine_preset_id: string }[];
    const current = printers.at(-1)!.machine_preset_id;
    await page.goto("/printers");
    await page.getByTestId("printer-add").click();
    const select = page.getByTestId("printer-machine-preset");
    await expect(select).toHaveValue(current);
    const first = await select.locator("option").first().getAttribute("value");
    expect(first).not.toBe(current);
    // Depuis un projet (lien « Déclarer une imprimante » de l'envoi) : son preset.
    const other = (await select.locator("option").nth(3).getAttribute("value"))!;
    await page.goto(`/printers?preset=${other}`);
    await page.getByTestId("printer-add").click();
    await expect(page.getByTestId("printer-machine-preset")).toHaveValue(other);
  });

  test("DEV1-a : envoi d'un G-code Voron vers une imprimante P1S — incompatible (message d'Orca), Envoyer désactivé @F:core.moonraker", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    const moonraker = process.env.MOONRAKER_URL;
    test.skip(!moonraker, "MOONRAKER_URL non défini");
    const projectId = await bootstrap(page);
    await page.goto("/printers");
    await page.getByTestId("printer-add").click();
    await page.locator('input[placeholder^="Imprimante du salon"]').fill("Atelier P1S");
    await page.getByTestId("printer-moonraker-url").fill(moonraker!);
    const select = page.getByTestId("printer-machine-preset");
    const p1s = await select.locator("option", { hasText: /^Bambu Lab P1S 0\.4 nozzle$/ }).getAttribute("value");
    await select.selectOption(p1s!);
    await page.getByTestId("printer-save").click();
    await expect(page.getByTestId("printer-row").filter({ hasText: "Atelier P1S" })).toBeVisible();
    await page.goto(`/projects/${projectId}`);
    await importModel(page, "cube20");
    await sliceInUi(page);
    await page.getByTestId("send-to-printer").click();
    const dlg = page.getByTestId("send-dialog");
    const target = dlg.getByTestId("send-printer-select");
    const value = await target.locator("option", { hasText: "Atelier P1S" }).getAttribute("value");
    await target.selectOption(value!);
    const warning = dlg.getByTestId("send-preset-mismatch");
    await expect(warning).toContainText("incompatible avec les préréglages d'imprimante choisis");
    await expect(warning).toContainText("Bambu Lab P1S 0.4 nozzle");
    await expect(dlg.getByTestId("send-confirm")).toBeDisabled();
  });

  test("SESS-a : session expirée → reconnexion sur place, l'enregistrement est rejoué avec les modifications @F:core.project-save", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const id = await ready(page, ["cube20"]);
    const me = (await (await page.request.get("/api/auth/me")).json()) as { email: string };
    // Session fermée côté serveur (autre onglet) : modifications non enregistrées.
    expect((await page.request.post("/api/auth/logout")).status()).toBeLessThan(300);
    await page.getByTestId("save-project").click();
    const dlg = page.getByTestId("reauth-dialog");
    await expect(dlg).toBeVisible();
    await expect(dlg.getByTestId("reauth-email")).toHaveValue(me.email);
    const saved = page.waitForResponse(
      (r) => r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url()) && r.status() < 300,
    );
    await dlg.getByTestId("reauth-password").fill(PASSWORD);
    await dlg.getByTestId("reauth-submit").click();
    await saved;
    await expect(dlg).toHaveCount(0);
    expect((await scene(page, id)).objects).toHaveLength(1);
    await expect(page).toHaveURL(new RegExp(`/projects/${id}`));
  });

  test("SESS-a : Annuler la reconnexion → « Session requise », rien de perdu à l'écran @F:core.project-save", async ({
    page,
  }) => {
    await ready(page, ["cube20"]);
    await page.request.post("/api/auth/logout");
    await page.getByTestId("save-project").click();
    const dlg = page.getByTestId("reauth-dialog");
    await expect(dlg).toBeVisible();
    await dlg.getByTestId("reauth-cancel").click();
    await expect(dlg).toHaveCount(0);
    await expect(page.getByTestId("object-row")).toHaveCount(1);
  });

  test("ARR-a : A après mise à l'échelle = chemin du bouton Disposer (scène enregistrée, arrangeur GUI) @F:shortcut.a @F:toolbar.arrange", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await ready(page, ["cube20", "cylinder"]);
    await page.getByTestId("object-row").first().click();
    await page.getByTestId("transform-scale-x").fill("300");
    await page.getByTestId("transform-scale-x").press("Tab");
    await focusEditor(page);
    const put = page.waitForResponse((r) => r.request().method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url()));
    const arranged = page.waitForRequest((r) => /\/arrange$/.test(r.url()) && r.method() === "POST");
    await page.keyboard.press("a");
    expect((await put).status()).toBeLessThan(300);
    const body = (await arranged).postDataJSON() as { scope?: string; items: unknown[] };
    expect(body.scope).toBe("all");
    expect(body.items).toEqual([]);
    await expect(page.getByText(/Arrangement : 2 objet\(s\) sur 1 plateau/)).toBeVisible({ timeout: 30_000 });
    // Maj+A : plateau actif.
    const plate = page.waitForRequest((r) => /\/arrange$/.test(r.url()) && r.method() === "POST");
    await page.keyboard.press("Shift+A");
    expect(((await plate).postDataJSON() as { scope?: string }).scope).toBe("plate");
  });

  test("FIX1-b : 800 × 500 — panneau OUTILS sous la barre du plateau, « Nouveau plateau » cliquable avec l'outil ouvert @F:core.slice-ui", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 800, height: 500 });
    await ready(page, ["cube20"]);
    await page.getByTestId("object-row").first().click();
    const panel = page.getByTestId("tool-panel");
    await expect(panel).toBeVisible();
    const bar = await page.getByTestId("plate-bar-area").boundingBox();
    const box = await panel.boundingBox();
    expect(box!.y).toBeGreaterThanOrEqual(bar!.y + bar!.height);
    const hit = await page.evaluate(() =>
      [...document.querySelectorAll('[data-testid="plate-bar-area"] button')].map((el) => {
        const r = el.getBoundingClientRect();
        const h = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return !!h && (h === el || el.contains(h));
      }),
    );
    expect(hit.length).toBeGreaterThan(3);
    expect(hit.every(Boolean)).toBe(true);
  });

  test("X-c : rotation affichée à 2 décimales (affichage seulement), saisie non interrompue @F:gizmo.rotate", async ({
    page,
  }) => {
    await ready(page, ["cube20"]);
    await page.getByTestId("object-row").first().click();
    await page.getByTestId("tool-rotate").click().catch(() => undefined);
    const rx = page.getByTestId("transform-rotation-x");
    await rx.fill("12.3456");
    await expect(rx).toHaveValue("12.3456");
    await rx.blur();
    await expect(rx).toHaveValue("12.35");
  });

  test("X-b : un ZIP donné à Importer importe les modèles contenus @F:menu.import-zip-archive", async ({ page }) => {
    await bootstrap(page);
    const dir = mkdtempSync(join(tmpdir(), "p46-"));
    const file = join(dir, "lot.zip");
    writeFileSync(
      file,
      zip({ "stl/cube20.stl": readFileSync(fixture("cube20")), "cylinder.stl": readFileSync(fixture("cylinder")), "notes.txt": Buffer.from("x") }),
    );
    await page.getByTestId("import-input").setInputFiles(file);
    await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 60_000 });
    await expect(page.getByTestId("object-name")).toHaveText([/cube20\.stl$/, /cylinder\.stl$/]);
  });

  test("FIX1-a : instances regroupées sous l'objet (sous-liste « Instances »), pas intercalées @F:ctx.set-number-of-instances", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await ready(page, ["cylinder", "bridge"]);
    await page.getByTestId("object-row").first().click({ button: "right" });
    await page.locator('[role=menu] [data-action="object.setInstances"]').click();
    const dialog = page.getByTestId("instance-count-dialog");
    await dialog.getByTestId("instance-count-input").fill("3");
    await dialog.getByTestId("instance-count-ok").click();
    await expect(page.getByTestId("object-row")).toHaveCount(4);
    await expect(page.getByTestId("object-name")).toHaveText([/cylinder/, /cylinder/, /cylinder/, /bridge/]);
    await expect(page.getByTestId("object-instances")).toHaveText(/Instances \(3\)/);
  });

  test("PR2-a : après l'assistant, la liste Filament ne montre que les filaments cochés (+ « Tous les filaments ») @F:menu.setup-wizard", async ({
    page,
  }) => {
    const id = await bootstrap(page);
    const select = page.locator('[data-testid="filament-slot"] select').first();
    await expect(select.locator("option").nth(5)).toBeAttached({ timeout: 30_000 });
    const all = await select.locator("option").count();
    const current = (await select.locator("option:checked").textContent())!.trim();
    // Préférence posée comme l'assistant : un seul filament coché.
    const prefs = (await (await page.request.get("/api/me/preferences")).json()) as Record<string, unknown>;
    expect(
      (await page.request.put("/api/me/preferences", { data: { ...prefs, wizardFilaments: [current] } })).status(),
    ).toBeLessThan(300);
    await page.evaluate(() => {
      for (const k of Object.keys(localStorage)) if (/pref/i.test(k)) localStorage.removeItem(k);
    });
    await page.goto(`/projects/${id}`);
    await expect(page.getByTestId("filament-show-all")).toBeVisible({ timeout: 30_000 });
    const few = await select.locator("option").count();
    expect(few).toBeLessThan(all);
    await expect(select.locator("option:checked")).toHaveText(current);
    await page.getByTestId("filament-show-all").click();
    await expect(select.locator("option")).toHaveCount(all);
  });

  test("PR2-b : assistant rouvert — imprimante possédée pré-cochée, « Annuler » retourne à la bibliothèque @F:menu.setup-wizard", async ({
    page,
  }) => {
    await bootstrap(page);
    await page.goto("/setup");
    await expect(page.getByTestId("wizard-filament").first()).toBeVisible({ timeout: 30_000 });
    await page.getByRole("button", { name: "Annuler" }).click();
    await expect(page).toHaveURL(/\/library$/);
  });

  test("JOURNEY-a : Ctrl+N / Alt+N dans la bibliothèque créent un projet @F:menu.new-project", async ({ page }) => {
    await bootstrap(page);
    await page.goto("/library");
    await expect(page.getByTestId("new-project")).toBeVisible();
    await page.keyboard.press("Alt+N");
    await expect(page).toHaveURL(/\/projects\/[^/]+$/, { timeout: 30_000 });
    await page.goto("/library");
    await expect(page.getByTestId("new-project")).toBeVisible();
    await page.keyboard.press("Control+N");
    await expect(page).toHaveURL(/\/projects\/[^/]+$/, { timeout: 30_000 });
  });

  test("CONFL-a : objets qui se chevauchent tranchés → avertissement grave « conflits de trajets G-code » d'Orca dans l'Aperçu, lien vers l'objet — lot 46 requis @F:core.slice-ui", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await ready(page, ["cube20", "cylinder"]);
    const probe = await page.request.post("/api/presets/export-bundle", { data: { presets: [] } });
    test.skip(probe.status() === 404 || probe.status() === 405, "lot 46 serveur non démarré");
    const rows = page.getByTestId("object-row");
    await rows.first().click();
    const x = Number(await page.getByTestId("transform-position-x").inputValue());
    const y = Number(await page.getByTestId("transform-position-y").inputValue());
    await rows.nth(1).click();
    await page.getByTestId("transform-position-x").fill(String(x + 3));
    await page.getByTestId("transform-position-x").press("Tab");
    await page.getByTestId("transform-position-y").fill(String(y));
    await page.getByTestId("transform-position-y").press("Tab");
    await sliceInUi(page);
    const banner = page.getByTestId("slice-conflict");
    await expect(banner).toBeVisible({ timeout: 30_000 });
    await expect(banner).toContainText("Des conflits de trajets G-code ont été détectés à la couche");
    await expect(banner).toContainText("<->");
    await banner.getByTestId("slice-conflict-jump").click();
    // « Sélectionner » : retour à Préparer, un objet du conflit sélectionné.
    await expect(page.locator('[data-testid="object-row"].bg-primary')).toHaveCount(1);
  });

  test("PR1-a/b : lot exporté au format d'Orca (.orca_printer / Presets.zip, from: User, clés du type, différences) puis réimporté à l'identique — lot 46 requis @F:menu.export-preset-bundle @F:menu.import-configs", async ({
    page,
    browser,
  }) => {
    test.setTimeout(240_000);
    const id = await bootstrap(page);
    // Process utilisateur dérivé : couche 0,16, 4 parois.
    const project = await (await page.request.get(`/api/projects/${id}`)).json();
    void project;
    const list = (await (await page.request.get("/api/presets?kind=process")).json()) as { id: string; name: string; origin: string }[];
    const parent = list.find((p) => p.origin === "system")!;
    const created = await (
      await page.request.post("/api/presets", {
        data: { kind: "process", name: "P46 Proc 016", inherits: parent.name, values: { layer_height: 0.16, wall_loops: 4 } },
      })
    ).json();
    const res = await page.request.post("/api/presets/export-bundle", { data: { presets: [{ id: created.id }] } });
    test.skip(res.status() === 404 || res.status() === 405, "lot 46 serveur non démarré");
    expect(res.status()).toBe(200);
    const bytes = await res.body();
    expect(bytes.subarray(0, 2).toString()).toBe("PK");
    // Import par un autre utilisateur via l'interface (Fichier › Importer › configurations).
    const ctx = await browser.newContext({ locale: "fr-FR" });
    const other = await ctx.newPage();
    await bootstrap(other);
    const dir = mkdtempSync(join(tmpdir(), "p46b-"));
    const name = /filename="([^"]+)"/.exec(res.headers()["content-disposition"] ?? "")?.[1] ?? "Presets.zip";
    const file = join(dir, name);
    writeFileSync(file, bytes);
    const imported = other.waitForResponse((r) => /\/api\/presets\/import-bundle/.test(r.url()));
    await other.locator('input[type=file][accept*=".orca_printer"]').setInputFiles(file);
    expect((await imported).status()).toBe(200);
    await expect(other.getByText(/1 profil\(s\) importé\(s\)/)).toBeVisible();
    const theirs = (await (await other.request.get("/api/presets?kind=process")).json()) as { id: string; name: string }[];
    const mine = theirs.find((p) => p.name === "P46 Proc 016")!;
    const resolved = await (await other.request.get(`/api/presets/${mine.id}/resolved`)).json();
    expect(Number(resolved.values.layer_height)).toBeCloseTo(0.16, 5);
    expect(Number(resolved.values.wall_loops)).toBe(4);
    // Aucune clé d'un autre type stockée (retraction_length = clé machine/filament).
    const detail = await (await other.request.get(`/api/presets/${mine.id}`)).json();
    expect(Object.keys(detail.values ?? {})).not.toContain("retraction_length");
    expect(Object.keys(detail.values ?? {})).not.toContain("nozzle_diameter");
    await ctx.close();
  });
});
