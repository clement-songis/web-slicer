#!/usr/bin/env python3
"""Scène de référence multi-filament (parité tour d'essuyage / purge /
changements d'outil), presets make_refs (Voron 2.4 350, 0.20 Standard,
Generic PLA) avec deux filaments (#F2754E, #FF0000) :

- two_objects.3mf : cube20 (extrudeur 1) + cylinder (extrudeur 2), tour
  d'essuyage (enable_prime_tower = 1) ; exporté par le CLI Orca
  (--export-3mf, sans tranchage) puis extrudeur de l'objet 2 posé à 2 ;
- two_volumes.3mf : cube20 seul, deux volumes pièce (le second, cylindre,
  sur l'extrudeur 2) — affectation d'extrudeur par volume ;
- *.orca.gcode : `orca-slicer --slice 0` du 3MF (config embarquée), variante
  majoritaire sur N tirages (tests/oracle/refs/README.md).
Usage : make_ref.py [N=5]
"""
import collections, hashlib, json, re, shutil, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
N = int(next((a for a in sys.argv[1:] if a.isdigit()), 5))
printer, process, filament = make_refs.Refs(make_refs.DEFAULT_PROFILES).trio()
proc = dict(process); proc["enable_prime_tower"] = "1"
f2 = dict(filament); f2["filament_colour"] = ["#FF0000"]


def settings_args(t: Path) -> list[str]:
    for n, d in (("machine", printer), ("process", proc), ("f1", filament), ("f2", f2)):
        (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
    return ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
            "--load-filaments", f"{t/'f1.json'};{t/'f2.json'}"]


def export_scene(models: list[str], dest: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        paths = []
        for m in models:
            shutil.copy(make_refs.ORACLE / "fixtures" / f"{m}.stl", t / f"{m}.stl")
            paths.append(str(t / f"{m}.stl"))
        subprocess.run(make_refs.orca_cmd() + settings_args(t) + ["--arrange", "1", "--orient", "0",
                       "--export-3mf", "scene.3mf", "--outputdir", str(t)] + paths,
                       capture_output=True, text=True, timeout=600, check=True)
        shutil.copy(t / "scene.3mf", dest)


def rewrite(src: Path, dest: Path, fn) -> None:
    z = zipfile.ZipFile(src)
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as out:
        for i in z.infolist():
            d = z.read(i.filename)
            if i.filename == "Metadata/model_settings.config":
                d = fn(d.decode()).encode()
            out.writestr(i, d)


def body_hash(text: str) -> str:
    return hashlib.md5("\n".join(l for l in text.split("\n") if not l.startswith(";")).encode()).hexdigest()[:8]


def reference(fixture: Path) -> None:
    draws = collections.Counter(); keep = {}
    for _ in range(N):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(out), str(fixture)],
                           capture_output=True, text=True, timeout=900)
            g = sorted(out.glob("*.gcode"))
            if not g:
                draws["ERROR"] += 1
                continue
            text = g[0].read_text()
            h = body_hash(text); draws[h] += 1; keep.setdefault(h, text)
    best, n = draws.most_common(1)[0]
    if best == "ERROR" or n * 2 <= N:
        raise SystemExit(f"{fixture.name} : pas de variante majoritaire {dict(draws)}")
    (HERE / fixture.name.replace(".3mf", ".orca.gcode")).write_text(keep[best])
    print(fixture.name, dict(draws), "->", best, f"{n}/{N}")


def mat(a):  # colonne-major -> lignes
    return [[a[c * 4 + r] for c in range(4)] for r in range(4)]


def flat(m):
    return [m[r][c] for c in range(4) for r in range(4)]


def mul(a, b):
    return [[sum(a[r][k] * b[k][c] for k in range(4)) for c in range(4)] for r in range(4)]


def inv_affine(m):
    r = [[m[i][j] for j in range(3)] for i in range(3)]
    det = (r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1]) - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
           + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]))
    inv = [[(r[(j + 1) % 3][(i + 1) % 3] * r[(j + 2) % 3][(i + 2) % 3] - r[(j + 1) % 3][(i + 2) % 3] * r[(j + 2) % 3][(i + 1) % 3]) / det
            for j in range(3)] for i in range(3)]
    t = [-sum(inv[i][k] * m[k][3] for k in range(3)) for i in range(3)]
    return [inv[0] + [t[0]], inv[1] + [t[1]], inv[2] + [t[2]], [0, 0, 0, 1]]


def two_volumes(src: Path, dest: Path) -> None:
    """Un seul objet (cube20) à deux volumes pièce : le cylindre devient une
    pièce de l'objet, sur l'extrudeur 2 (affectation d'extrudeur par volume).
    Construit par engine-worker (load-model-raw / export-project-3mf, donc
    store_bbs_3mf d'Orca), config = celle de two_objects.3mf."""
    worker = Path(__file__).resolve().parents[3].parent / "target/debug/engine-worker"
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        subprocess.run([str(worker), "load-model-raw", str(src), "3mf", str(t / "m.json")], check=True,
                       capture_output=True)
        m = json.loads((t / "m.json").read_text())
        a, b = m["objects"]
        ma, mb = mat(a["instances"][0]["matrix"]), mat(b["instances"][0]["matrix"])
        vol = dict(b["volumes"][0])
        vol["matrix"] = flat(mul(inv_affine(ma), mul(mb, mat(vol["matrix"]))))
        vol["extruder"] = 2
        a["volumes"][0]["extruder"] = 1
        a["volumes"].append(vol)
        model = {"objects": [a]}
        req = {"model": model, "config": {}, "plates": [], "output_path": str(t / "out.3mf")}
        (t / "req.json").write_text(json.dumps(req))
        subprocess.run([str(worker), "export-project-3mf", str(t / "req.json")], check=True, capture_output=True)
        cfg = zipfile.ZipFile(src).read("Metadata/project_settings.config")
        z = zipfile.ZipFile(t / "out.3mf")
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as out:
            for i in z.infolist():
                d = z.read(i.filename)
                if i.filename == "Metadata/project_settings.config":
                    d = cfg
                out.writestr(i, d)


def mmu_painted(src: Path, dest: Path) -> None:
    """cube20 seul, moitié haute PEINTE en filament 2 (peinture MMU, canal
    mmu_segmentation : état « 8 » = extrudeur 2 dans la sérialisation
    TriangleSelector), le reste sur le filament 1. Écrit par engine-worker
    (store_bbs_3mf), config = celle de two_objects.3mf."""
    worker = Path(__file__).resolve().parents[3].parent / "target/debug/engine-worker"
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        subprocess.run([str(worker), "load-model-raw", str(src), "3mf", str(t / "m.json")], check=True,
                       capture_output=True)
        m = json.loads((t / "m.json").read_text())
        cube = m["objects"][0]
        vol = cube["volumes"][0]
        vol["extruder"] = 1
        verts = vol["mesh"]["vertices"]
        zs = [v[2] for v in verts]
        mid = (min(zs) + max(zs)) / 2
        facets = []
        for i, tri in enumerate(vol["mesh"]["indices"]):
            if sum(verts[k][2] for k in tri) / 3 > mid:
                facets.append(["mmu", i, "8"])
        vol["paint"] = {"facets": facets}
        req = {"model": {"objects": [cube]}, "config": {}, "plates": [], "output_path": str(t / "out.3mf")}
        (t / "req.json").write_text(json.dumps(req))
        subprocess.run([str(worker), "export-project-3mf", str(t / "req.json")], check=True, capture_output=True)
        cfg = zipfile.ZipFile(src).read("Metadata/project_settings.config")
        z = zipfile.ZipFile(t / "out.3mf")
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as out:
            for i in z.infolist():
                d = z.read(i.filename)
                if i.filename == "Metadata/project_settings.config":
                    d = cfg
                out.writestr(i, d)


def dump_presets() -> None:
    """Presets aplatis (mêmes que la scène) pour le test de fusion backend."""
    d = HERE / "presets"; d.mkdir(exist_ok=True)
    for n, p in (("printer", printer), ("process", proc), ("filament", filament), ("filament2", f2)):
        (d / f"{n}.json").write_text(json.dumps(p, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    dump_presets()
    if "--presets-only" in sys.argv:
        raise SystemExit(0)
    raw = HERE / "_scene.3mf"
    if "--mmu-only" in sys.argv:
        mmu_painted(HERE / "two_objects.3mf", HERE / "mmu_painted.3mf")
        reference(HERE / "mmu_painted.3mf")
        raise SystemExit(0)
    if "--volumes-only" in sys.argv:
        two_volumes(HERE / "two_objects.3mf", HERE / "two_volumes.3mf")
        reference(HERE / "two_volumes.3mf")
        raise SystemExit(0)
    export_scene(["cube20", "cylinder"], raw)
    rewrite(raw, HERE / "two_objects.3mf", lambda s: re.sub(
        r'(<object id="4">\s*<metadata key="name" value="cylinder.stl"/>\s*<metadata key="extruder" value=")1',
        r"\g<1>2", s))
    raw.unlink()
    two_volumes(HERE / "two_objects.3mf", HERE / "two_volumes.3mf")
    for f in ("two_objects.3mf", "two_volumes.3mf"):
        if "--only" in sys.argv and f not in sys.argv:
            continue
        reference(HERE / f)
