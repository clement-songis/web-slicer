// Phase 50 — correctifs de l'audit 48, partie interface :
// - DLG48-a : `timelapse_type` exposé par plateau (dialogue « Réglages du
//   plateau » + panneau inline de l'onglet Objets), persisté dans
//   `plates[].settings` et porté par la requête de tranchage (scène).
// - DLG48-b : préférence « Vignettes G-code » (liste Orca `thumbnails`,
//   vide = défauts 48x48/PNG,300x300/PNG jamais écrits) qui rejoint le
//   tranchage comme surcharge printer (thumbnails + thumbnails_format).
// - FIX1-a : « Remplir le plateau d'instances… » crée des instances du même
//   objet (groupe `instanceGroup`, FillBedJob `m_instances` vrai), plus
//   d'objets frères « (copie) » ; « … de copies » garde des objets séparés
//   (FillBedJob faux, parité Orca).
// - D2 : un menu ouvert + survol d'un autre titre de la barre ⇒ ouverture
//   immédiate (menus natifs d'Orca/wx).
//
// Oracle Orca 2.4.1 : PrintConfig.cpp:5879-5894 (`timelapse_type`, coEnum
// Traditional/Smooth), FillBedJob.cpp:175/190-198 (copies vs instances),
// Plater.cpp:14762-14776, Tab.cpp:4506-4533 (`thumbnails` machine,
// `thumbnails_format` dérivée de la première entrée), menus wx de la barre
// (ouverture au survol une fois la barre active).
//
// CONTRAT: plate-setting-timelapse_type, settings-dialog [data-param],
// settings-apply, pref-tab-behaviour, pref-gcode-thumbnails, slice-button,
// object-row, object-instances, plate-settings-button.
import { expect, test, type Page } from "@playwright/test";
import { bootstrap, importModel } from "../lib/app";

type Plate = { id: string; settings?: Record<string, unknown> };
type Project = { id: string; scene: { plates: { plates: Plate[] } | Plate[] } };
type SliceBody = {
  scene?: {
    plates?:
      | { plates?: { settings?: Record<string, unknown> }[] }
      | { settings?: Record<string, unknown> }[];
  };
  active_presets?: { overrides?: { printer?: Record<string, unknown> } };
};

const rows = (page: Page) => page.getByTestId("object-row");

async function save(page: Page) {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "PUT" &&
        /\/api\/projects\/[^/]+$/.test(r.url()),
    ),
    page.getByTestId("save-project").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
}

async function project(page: Page, id: string): Promise<Project> {
  const res = await page.request.get(`/api/projects/${id}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as Project;
}

function platesOf(p: Project): Plate[] {
  return Array.isArray(p.scene.plates) ? p.scene.plates : p.scene.plates.plates;
}

/** G-code du dernier tranchage déclenché par le bouton (réponse /slice). */
async function sliceViaButton(
  page: Page,
): Promise<{ body: SliceBody; gcode: string }> {
  const [res] = await Promise.all([
    page.waitForResponse(
      (r) =>
        r.request().method() === "POST" &&
        /\/api\/projects\/[^/]+\/slice$/.test(r.url()),
    ),
    page.getByTestId("slice-button").click(),
  ]);
  expect(res.status(), await res.text()).toBeLessThan(300);
  const body = (await res.request().postDataJSON()) as SliceBody;
  const job = (await res.json()) as { jobs: { id: string }[] };
  let gcodeId: string | null = null;
  await expect
    .poll(
      async () => {
        const j = (await (
          await page.request.get(`/api/jobs/${job.jobs[0].id}`)
        ).json()) as {
          status: string;
          gcode_id?: string;
        };
        if (j.status === "failed") throw new Error("tranchage en échec");
        gcodeId = j.gcode_id ?? null;
        return j.status;
      },
      { timeout: 170_000, intervals: [1000] },
    )
    .toMatch(/^(done|completed|succeeded)$/);
  const g = await page.request.get(`/api/gcodes/${gcodeId}/download`);
  expect(g.status()).toBe(200);
  return { body, gcode: await g.text() };
}

async function objectMenu(page: Page, name: RegExp, row = 0) {
  await rows(page).nth(row).click({ button: "right" });
  const item = page.getByRole("menuitem", { name });
  await item.scrollIntoViewIfNeeded();
  await item.click();
}

test("DLG48-a : timelapse_type par plateau — panneau inline, dialogue, persistance, requête de tranchage @F:editor.plate-settings @F:param.timelapse", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });
  await page.getByRole("button", { name: "Objets", exact: true }).click();

  // Panneau inline : l'option Smooth (1) du coEnum d'Orca.
  const panel = page.getByTestId("plate-settings");
  const select = panel.getByTestId("plate-setting-timelapse_type");
  await expect(select).toBeVisible();
  await expect(select.locator("option")).toHaveCount(3); // Profil + Traditional + Smooth
  await select.selectOption("1");
  await save(page);
  let p = await project(page, id);
  expect(platesOf(p)[0].settings?.timelapse_type).toBe("1");

  // Dialogue « Réglages du plateau » : même clé, valeur reprise, options Orca.
  await page.getByTestId("plate-settings-button").click();
  const dialog = page.getByTestId("settings-dialog");
  await expect(dialog).toBeVisible();
  const row = dialog.locator('[data-param="timelapse_type"]');
  await expect(row.locator("select")).toHaveValue("1");
  await row.locator("select").selectOption("");
  await dialog.getByTestId("settings-apply").click();
  await save(page);
  p = await project(page, id);
  // Retour au profil : plus de surcharge du plateau.
  expect(platesOf(p)[0].settings?.timelapse_type).toBeUndefined();

  // La surcharge du plateau atteint la requête de tranchage (scène) :
  // trancher sans enregistrer (projet « sale ») envoie la scène affichée.
  await select.selectOption("1");
  const { body, gcode } = await sliceViaButton(page);
  const plate = Array.isArray(body.scene?.plates)
    ? body.scene?.plates?.[0]
    : body.scene?.plates?.plates?.[0];
  expect(plate?.settings?.timelapse_type).toBe("1");
  // Le bloc de config du G-code porte la clé (valeur du profil ici ;
  // l'injection par plateau côté moteur suit la liste blanche engine
  // PLATE_SETTING_KEYS — voir rapport phase 50).
  expect(new RegExp("^; timelapse_type = .+$", "m").test(gcode)).toBe(true);
});

test("DLG48-b : préférence « Vignettes G-code » — vide = défauts (jamais écrits), 96x96/PNG tranche avec surcharge @F:param.thumbnails @F:pref.gcode-thumbnails", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });

  // Référence : préférence vide → défauts du moteur dans le G-code, aucune
  // surcharge écrite dans la requête.
  const first = await sliceViaButton(page);
  expect(first.gcode).toMatch(/; thumbnail begin 48x48/);
  expect(first.gcode).toMatch(/; thumbnail begin 300x300/);
  expect(
    first.body.active_presets?.overrides?.printer?.thumbnails,
  ).toBeUndefined();

  // Préférence (menu Fichier ▸ Préférences, onglet Comportement) : 96x96/PNG.
  await page.getByRole("button", { name: /^Fichier$|^File$/ }).click();
  await page
    .getByRole("menuitem", { name: /Préférences|Preferences/ })
    .click();
  await page.getByTestId("pref-tab-behaviour").click();
  const field = page.getByTestId("pref-gcode-thumbnails");
  await expect(field).toBeVisible();
  await expect(field).toHaveAttribute("placeholder", "48x48/PNG,300x300/PNG");
  await field.fill("96x96/PNG");
  // Tab : valide (change) et garde le focus dans le dialogue ; Escape ferme.
  await field.press("Tab");
  await page.keyboard.press("Escape");

  const second = await sliceViaButton(page);
  expect(second.body.active_presets?.overrides?.printer).toEqual({
    thumbnails: "96x96/PNG",
    thumbnails_format: "PNG",
  });
  expect(second.gcode).toMatch(/; thumbnail begin 96x96/);
  expect(second.gcode).not.toMatch(/; thumbnail begin 48x48/);

  // Retour à vide : les défauts reviennent, rien n'est écrit.
  await page.getByRole("button", { name: /^Fichier$|^File$/ }).click();
  await page
    .getByRole("menuitem", { name: /Préférences|Preferences/ })
    .click();
  await page.getByTestId("pref-tab-behaviour").click();
  const clear = page.getByTestId("pref-gcode-thumbnails");
  await clear.fill("  ");
  await clear.press("Tab");
  await page.keyboard.press("Escape");
  const third = await sliceViaButton(page);
  expect(
    third.body.active_presets?.overrides?.printer?.thumbnails,
  ).toBeUndefined();
  expect(third.gcode).toMatch(/; thumbnail begin 48x48/);
  expect(id).toBeTruthy();
});

test("FIX1-a : « Remplir le plateau d'instances… » regroupe les copies sous l'objet (instances d'Orca), « … de copies » reste des objets séparés @F:ctx.fill-bed-with-instances @F:ctx.fill-bed-with-copies", async ({
  page,
}) => {
  const id = await bootstrap(page);
  await importModel(page, "cube20");
  await expect(rows(page)).toHaveCount(1, { timeout: 60_000 });
  await expect(page.getByTestId("import-status")).toHaveCount(0, {
    timeout: 60_000,
  });

  // Instances : regroupées sous l'objet — bloc « Instances (N) », lignes
  // d'instances portant toutes le nom de la source (aucun « (copie) »).
  await objectMenu(
    page,
    /^Fill bed with instances|^Remplir le plateau d.instances/,
  );
  await expect
    .poll(() => page.getByTestId("object-instances").count(), {
      timeout: 30_000,
    })
    .toBeGreaterThan(0);
  const names = await page.getByTestId("object-name").allInnerTexts();
  expect(names.length).toBeGreaterThan(1);
  expect(names.some((n) => n.includes("(copie)"))).toBe(false);
  await save(page);
  const scene = (await project(page, id)).scene as unknown as {
    objects: unknown[];
  };
  expect(scene.objects.length).toBeGreaterThan(1);

  // Copies (parité Orca FillBedJob faux) : objets frères « (copie) ».
  await objectMenu(page, /^Fill bed with copies|^Remplir le plateau de copies/);
  await expect
    .poll(
      async () => {
        const n = await page.getByTestId("object-name").allInnerTexts();
        return n.filter((x) => x.includes("(copie)")).length;
      },
      { timeout: 30_000 },
    )
    .toBeGreaterThan(0);
});

test("D2 : la barre de menus ouvre les menus au survol une fois active @F:menu.hover", async ({
  page,
}) => {
  await bootstrap(page);
  await page.getByRole("button", { name: /^Fichier$|^File$/ }).click();
  await expect(
    page.getByRole("menuitem", { name: /Nouveau projet|New Project/ }),
  ).toBeVisible();
  // Survol d'un autre titre : le menu bascule sans clic (menus wx d'Orca).
  // mouse.move (vrai pointeur) : le menu ouvert peut couvrir le bouton.
  const editBtn = page.getByRole("button", { name: /^Édition$|^Edit$/ });
  const box = await editBtn.boundingBox();
  expect(box).toBeTruthy();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  await expect(
    page.getByRole("menuitem", { name: /Annuler|Undo/ }),
  ).toBeVisible();
  await expect(
    page.getByRole("menuitem", { name: /Nouveau projet|New Project/ }),
  ).toHaveCount(0);
});
