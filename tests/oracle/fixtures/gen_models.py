#!/usr/bin/env python3
"""Génère le corpus de modèles STL (binaires) des tests oracle. Déterministe."""
from __future__ import annotations

import math
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent
Tri = tuple[tuple[float, float, float], tuple[float, float, float], tuple[float, float, float]]


def box(x0, y0, z0, x1, y1, z1) -> list[Tri]:
    v = [(x, y, z) for z in (z0, z1) for y in (y0, y1) for x in (x0, x1)]
    faces = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    tris: list[Tri] = []
    for a, b, c, d in faces:
        tris += [(v[a], v[b], v[c]), (v[a], v[c], v[d])]
    return tris


def frustum(r_bot: float, r_top: float, h: float, n: int, square: bool = False) -> list[Tri]:
    """Tronc (cylindre si r_bot == r_top) ; `square` → base carrée (n=4, demi-côté r)."""
    if square:
        n = 4
    ang = [2 * math.pi * i / n + (math.pi / 4 if square else 0) for i in range(n)]
    k = math.sqrt(2) if square else 1.0
    bot = [(k * r_bot * math.cos(a), k * r_bot * math.sin(a), 0.0) for a in ang]
    top = [(k * r_top * math.cos(a), k * r_top * math.sin(a), h) for a in ang]
    cb, ct = (0.0, 0.0, 0.0), (0.0, 0.0, h)
    tris: list[Tri] = []
    for i in range(n):
        j = (i + 1) % n
        tris += [(cb, bot[j], bot[i]), (ct, top[i], top[j]), (bot[i], bot[j], top[j]), (bot[i], top[j], top[i])]
    return tris


def normal(t: Tri):
    (ax, ay, az), (bx, by, bz), (cx, cy, cz) = t
    ux, uy, uz, vx, vy, vz = bx - ax, by - ay, bz - az, cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    ln = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
    return nx / ln, ny / ln, nz / ln


def write(name: str, tris: list[Tri]) -> None:
    with open(OUT / f"{name}.stl", "wb") as fh:
        fh.write(name.encode().ljust(80, b"\0"))
        fh.write(struct.pack("<I", len(tris)))
        for t in tris:
            fh.write(struct.pack("<3f", *normal(t)))
            for p in t:
                fh.write(struct.pack("<3f", *p))
            fh.write(b"\0\0")


MODELS = {
    "cube20": box(0, 0, 0, 20, 20, 20),
    "cylinder": frustum(10, 10, 20, 64),
    "overhang": frustum(5, 15, 20, 4, square=True),
    "bridge": box(0, 0, 0, 10, 10, 15) + box(40, 0, 0, 50, 10, 15) + box(0, 0, 15, 50, 10, 18),
    "thinwall": box(0, 0, 0, 40, 1.2, 20),
    "twoparts": box(0, 0, 0, 15, 15, 10) + box(25, 0, 0, 40, 15, 10),
}

if __name__ == "__main__":
    for name, tris in MODELS.items():
        write(name, tris)
        print(f"{name}.stl : {len(tris)} triangles")
