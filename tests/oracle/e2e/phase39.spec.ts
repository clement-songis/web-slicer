// Phase 39 — derniers manques d'interface du lot 39 : alerte d'erreurs de
// maillage avant le premier enregistrement (POST mesh-errors), échelle groupée
// des pièces coupées, vue d'assemblage (coupe, mode Objet / Pièce, infos),
// fil de fer de la peinture (Alt+Maj+Entrée), interface web des hôtes autres
// que Moonraker dans l'onglet Appareil, SSAO de la vue réaliste.
import { expect, test, type Page, type Request } from "@playwright/test";
import { createServer } from "node:http";
import { bootstrap, importModel } from "../lib/app";

test.use({ locale: "fr-FR" });

async function ready(page: Page, models = ["cube20"]) {
  const id = await bootstrap(page);
  for (const m of models) await importModel(page, m);
  await expect(page.getByTestId("import-status")).toHaveCount(0, { timeout: 60_000 });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  return id;
}

async function saveNow(page: Page) {
  const [req] = await Promise.all([
    page.waitForRequest(
      (r: Request) => r.method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  const res = await req.response();
  expect(res?.status()).toBeLessThan(300);
}

async function focusEditor(page: Page) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.locator("body").focus();
}

/** Cube de 20 mm à une facette manquante (3 arêtes ouvertes). */
function brokenCube(): Buffer {
  const v = (x: number, y: number, z: number) => [x * 20, y * 20, z * 20];
  const quads = [
    [v(0, 0, 0), v(1, 0, 0), v(1, 1, 0), v(0, 1, 0)],
    [v(0, 0, 1), v(0, 1, 1), v(1, 1, 1), v(1, 0, 1)],
    [v(0, 0, 0), v(0, 0, 1), v(1, 0, 1), v(1, 0, 0)],
    [v(0, 1, 0), v(1, 1, 0), v(1, 1, 1), v(0, 1, 1)],
    [v(0, 0, 0), v(0, 1, 0), v(0, 1, 1), v(0, 0, 1)],
    [v(1, 0, 0), v(1, 0, 1), v(1, 1, 1), v(1, 1, 0)],
  ];
  const tris = quads.flatMap(([a, b, c, d]) => [[a, c, b], [a, d, c]]).slice(1);
  const buf = Buffer.alloc(84 + tris.length * 50);
  buf.writeUInt32LE(tris.length, 80);
  tris.forEach((t, i) => {
    const o = 84 + i * 50 + 12;
    t.forEach((p, k) => p.forEach((c, j) => buf.writeFloatLE(c, o + k * 12 + j * 4)));
  });
  return buf;
}

test("erreurs de maillage : icône dès l'import, avant tout enregistrement (POST de la scène courante) @F:editor.mesh-repair @F:ctx.fix-model", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await bootstrap(page);
  const puts: string[] = [];
  page.on("request", (r) => {
    if (r.method() === "PUT" && /\/api\/projects\/[^/]+$/.test(r.url())) puts.push(r.url());
  });
  const post = page.waitForRequest(
    (r) => r.method() === "POST" && /\/api\/projects\/[^/]+\/mesh-errors$/.test(r.url()),
    { timeout: 60_000 },
  );
  await page.getByTestId("import-input").setInputFiles({
    name: "holed.stl",
    mimeType: "model/stl",
    buffer: brokenCube(),
  });
  await expect(page.getByTestId("object-row")).toHaveCount(1, { timeout: 60_000 });
  const req = await post;
  const body = req.postDataJSON() as { objects: { id: string; modelId?: string }[] };
  expect(body.objects).toHaveLength(1);
  expect(body.objects[0].modelId).toBeTruthy();
  const warning = page.getByTestId("object-mesh-warning");
  await expect(warning).toBeVisible({ timeout: 30_000 });
  await expect(warning).toHaveAttribute("data-open-edges", "3");
  await expect(warning).toHaveAttribute("data-tip", /3 arêtes non-manifold/);
  // Aucun enregistrement n'a été nécessaire.
  expect(puts).toHaveLength(0);
  await expect(page.getByTestId("save-project")).toBeVisible();
});

test("échelle groupée : l'outil Échelle met à l'échelle toutes les pièces d'une coupe autour de leur centre @F:gizmo.cut @F:gizmo.scale", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const id = await ready(page, ["cube20", "cube20"]);
  await saveNow(page);
  const project = await (await page.request.get(`/api/projects/${id}`)).json();
  // Deux pièces d'une même coupe, côte à côte (x = 100 et x = 140).
  project.scene.objects.forEach((o: { cutId?: unknown; transform: { position: number[] } }, i: number) => {
    o.cutId = { id: 7, checkSum: 2, connectorsCnt: 0 };
    o.transform.position = [100 + 40 * i, 100, 10];
  });
  const put = await page.request.put(`/api/projects/${id}`, {
    data: { expected_version: Number(project.version), scene: project.scene, active_presets: project.active_presets },
  });
  expect(put.status()).toBeLessThan(300);
  await page.reload();
  await expect(page.getByTestId("object-row")).toHaveCount(2, { timeout: 60_000 });
  await page.getByRole("button", { name: "Objets", exact: true }).click();
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-scale").click();
  await expect(page.getByTestId("cut-scale-group")).toContainText("2 pièces de la coupe");
  await page.getByLabel("Échelle X", { exact: true }).fill("2");
  await saveNow(page);
  const saved = await (await page.request.get(`/api/projects/${id}`)).json();
  const objs = saved.scene.objects as { transform: { position: number[]; scale: number[] } }[];
  for (const o of objs) expect(o.transform.scale).toEqual([2, 2, 2]);
  const xs = objs.map((o) => o.transform.position[0]).sort((a, b) => a - b);
  // Centre commun x = 120 : 120 ± 2 × 20 → 80 et 160 (écart doublé), reposées (z = 20).
  expect(xs[0]).toBeCloseTo(80, 3);
  expect(xs[1]).toBeCloseTo(160, 3);
  for (const o of objs) {
    expect(o.transform.position[1]).toBeCloseTo(100, 3);
    expect(o.transform.position[2]).toBeCloseTo(20, 3);
  }
});

test("vue d'assemblage : vue en coupe, mode de sélection Objet / Pièce, informations @F:gizmo.assembly @F:toolbar.assembly-view", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await ready(page, ["cube20", "cylinder"]);
  const canvas = page.getByTestId("scene-canvas");
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("toolbar-assembly_view").click();
  const info = page.getByTestId("assembly-info");
  await expect(info).toContainText("Informations sur l'assemblage");
  await expect(page.getByTestId("assembly-info-size")).toHaveText("20.00 x 20.00 x 20.00 mm");
  await expect(page.getByTestId("assembly-info-volume")).toHaveText("8000.00 mm³");
  // Vue en coupe : toute la scène est coupée (plan face à la vue).
  await expect(canvas).not.toHaveAttribute("data-clip-all", "on");
  await page.getByLabel("Position de la coupe").fill("0.5");
  await page.getByLabel("Position de la coupe").press("Enter");
  await expect(page.getByTestId("assembly-section")).toHaveAttribute("data-section", "0.50");
  await expect(canvas).toHaveAttribute("data-clip-all", "on");
  await expect(canvas).toHaveAttribute("data-section-offset", /^-?\d+\.\d\d$/);
  await page.getByTestId("assembly-section-reset").click();
  await expect(canvas).toHaveAttribute("data-clip-all", "on");
  // Mode Pièce : le clic choisit la pièce (Alt implicite).
  await page.getByTestId("assembly-select-part").click();
  await expect(page.getByTestId("assembly-select-part")).toHaveAttribute("aria-checked", "true");
  await expect(canvas).toHaveAttribute("data-part-select", "on");
  // Sortie de la vue : coupe et mode Pièce retirés (comme Orca hors de la vue).
  await page.getByTestId("toolbar-assembly_view").click();
  await expect(canvas).not.toHaveAttribute("data-clip-all", "on");
  await expect(canvas).not.toHaveAttribute("data-part-select", "on");
});

test("peinture multi-matériaux : fil de fer Alt+Maj+Entrée (Toggle Wireframe d'Orca) @F:gizmo.mmsegmentation", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await ready(page);
  const canvas = page.getByTestId("scene-canvas");
  await page.getByTestId("object-name").first().click();
  await page.getByTestId("tool-mm-paint").click();
  const box = page.getByTestId("paint-wireframe");
  await expect(box).not.toBeChecked();
  await expect(canvas).not.toHaveAttribute("data-wireframe", "on");
  await focusEditor(page);
  await page.keyboard.press("Alt+Shift+Enter");
  await expect(canvas).toHaveAttribute("data-wireframe", "on");
  await expect(box).toBeChecked();
  await page.keyboard.press("Control+Shift+Enter");
  await expect(canvas).not.toHaveAttribute("data-wireframe", "on");
  await box.check();
  await expect(canvas).toHaveAttribute("data-wireframe", "on");
  // Orca ne dessine le fil de fer qu'en peinture multi-matériaux.
  await page.getByTestId("tool-support-paint").click();
  await expect(canvas).not.toHaveAttribute("data-wireframe", "on");
});

/** Interface web simulée d'un hôte OctoPrint (page HTML intégrable). */
function mockWebUi(): Promise<{ url: string; close: () => void }> {
  return new Promise((ok) => {
    const srv = createServer((req, res) => {
      if (req.url?.startsWith("/api/")) {
        res.writeHead(404, { "content-type": "application/json" });
        return res.end("{}");
      }
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      res.end("<!doctype html><title>OctoPrint</title><h1>Interface OctoPrint simulée</h1>");
    });
    srv.listen(0, "127.0.0.1", () => {
      const a = srv.address() as { port: number };
      ok({ url: "http://127.0.0.1:" + a.port, close: () => srv.close() });
    });
  });
}

test("onglet Appareil : interface web d'un hôte OctoPrint intégrée, repli lien externe @F:ui.device @F:ui.print-hosts", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const host = await mockWebUi();
  try {
    await bootstrap(page);
    const existing = await (await page.request.get("/api/printers")).json();
    const created = await page.request.post("/api/printers", {
      data: {
        name: "OctoPi phase 39",
        moonraker_url: host.url,
        host_type: "octoprint",
        api_key: "CLE",
        machine_preset_id: existing[0].machine_preset_id,
      },
    });
    expect(created.status()).toBe(201);
    await page.getByRole("button", { name: /^(appareil|device)$/i }).first().click();
    const panel = page.getByTestId("device-panel");
    await panel.getByText("OctoPi phase 39").first().click();
    const webui = panel.getByTestId("device-webui");
    await expect(webui).toHaveAttribute("data-webui-url", host.url);
    await expect(webui).toHaveAttribute("data-webui-state", "embedded", { timeout: 20_000 });
    await expect(
      page.frameLocator('[data-testid="device-webui-frame"]').getByRole("heading"),
    ).toHaveText("Interface OctoPrint simulée");
    await expect(webui.getByTestId("device-webui-open")).toHaveAttribute("href", host.url);
    // « Lien seulement » : repli sur le lien externe, mémorisé.
    await webui.getByTestId("device-webui-embed-toggle").click();
    await expect(webui).toHaveAttribute("data-webui-state", "link:manual");
    await expect(webui.getByTestId("device-webui-fallback").getByRole("link")).toHaveAttribute("href", host.url);
    await webui.getByTestId("device-webui-embed-toggle").click();
    await expect(webui).toHaveAttribute("data-webui-state", "embedded", { timeout: 20_000 });
    // Adresse saisie (sans schéma → http://, comme get_print_host_webui).
    await webui.getByTestId("device-webui-edit").click();
    await webui.getByTestId("device-webui-url").fill(host.url.replace("http://", "") + "/ui");
    await webui.getByRole("button", { name: "Enregistrer" }).click();
    await expect(webui).toHaveAttribute("data-webui-url", host.url + "/ui");
  } finally {
    host.close();
  }
});

test("vue réaliste : SSAO (préférence d'Orca, passe d'écran sans erreur de shader) @F:editor.menu-view-prefs @F:menu.preferences", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const errors: string[] = [];
  page.on("console", (m) => {
    if (/Shader Error|WebGLProgram|GL_INVALID|WebGL: INVALID/i.test(m.text())) errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  await ready(page);
  const canvas = page.getByTestId("scene-canvas");
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  const prefs = page.getByTestId("preferences-dialog");
  await prefs.getByTestId("pref-tab-graphics").click();
  await expect(prefs.getByTestId("pref-realistic-ssao")).toBeDisabled();
  await prefs.getByTestId("pref-realistic-view").check();
  await expect(prefs.getByTestId("pref-realistic-ssao")).not.toBeChecked();
  await prefs.getByTestId("pref-realistic-ssao").check();
  await page.getByTestId("preferences-close").click();
  await expect(canvas).toHaveAttribute("data-realistic", "realistic phong ssao");
  // Quelques images rendues avec la passe (orbite), puis retour sans SSAO.
  const b = (await canvas.boundingBox())!;
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2 + 60, b.y + b.height / 2 + 20, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(500);
  await focusEditor(page);
  await page.keyboard.press("Control+p");
  await prefs.getByTestId("pref-tab-graphics").click();
  await prefs.getByTestId("pref-realistic-ssao").uncheck();
  await page.getByTestId("preferences-close").click();
  await expect(canvas).toHaveAttribute("data-realistic", "realistic phong");
  expect(errors).toEqual([]);
});
