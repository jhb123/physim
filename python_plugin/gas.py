"""
Gas simulation logic for physim's `gas` transform element.

physim calls into this module through python_plugin/plugin.c, which embeds
a CPython interpreter inside a compiled dylib/so (physim only loads plugins
that satisfy its C ABI, so this file is never `dlopen`'d directly).

The physics: a Lennard-Jones pair potential between every pair of entities,
which is the standard way to model a gas as colliding/jostling particles
(short-range repulsion when particles get close, weak attraction just
beyond that, no force past the cutoff).

    V(r) = 4*epsilon * ((sigma/r)^12 - (sigma/r)^6)

This module has no dependency on physim_core; it only knows about plain
tuples of floats, so it can be tested/tuned standalone with `python3 -i gas.py`.
"""

import json
import math

try:
    import numpy as np

    _HAVE_NUMPY = True
except ImportError:  # numpy is optional; fall back to a pure-python path
    _HAVE_NUMPY = False

DEFAULT_EPSILON = 1.0
DEFAULT_SIGMA = 0.05
DEFAULT_CUTOFF_FACTOR = 2.5


def init(config_str):
    """Called once per element instance. config_str is the raw JSON blob
    physim passes in from the pipeline's TOML (e.g. `[[elements.gas]]`)."""
    try:
        raw = json.loads(config_str) if config_str else {}
    except json.JSONDecodeError:
        raw = {}

    sigma = float(raw.get("sigma", DEFAULT_SIGMA))
    return {
        "epsilon": float(raw.get("epsilon", DEFAULT_EPSILON)),
        "sigma": sigma,
        "cutoff": sigma * float(raw.get("cutoff", DEFAULT_CUTOFF_FACTOR)),
        # Clamp the minimum separation used in the force calculation to sigma
        # itself, i.e. cap the repulsive force at its value at contact
        # distance. Random initial placement can put particles arbitrarily
        # close together, and the raw r^-12 repulsive term blows up fast
        # enough as r -> 0 that a single timestep can eject a particle at
        # an enormous velocity. This is a standard soft-core trick to keep
        # explicit integrators (like physim's `euler`) stable.
        "min_r": sigma,
    }


def transform(cfg, entities):
    """entities is a list of (x, y, z, vx, vy, vz, radius, mass) tuples.
    Returns a list of (ax, ay, az) tuples, one per entity, in the same order."""
    if not entities:
        return []
    if _HAVE_NUMPY:
        return _transform_numpy(cfg, entities)
    return _transform_pure_python(cfg, entities)


def _transform_numpy(cfg, entities):
    arr = np.asarray(entities, dtype=np.float64)
    pos = arr[:, 0:3]
    mass = arr[:, 7]

    epsilon = cfg["epsilon"]
    sigma = cfg["sigma"]
    cutoff = cfg["cutoff"]
    min_r2 = cfg["min_r"] ** 2

    diff = pos[:, None, :] - pos[None, :, :]  # diff[i, j] = pos_i - pos_j
    r2 = np.sum(diff * diff, axis=-1)
    np.fill_diagonal(r2, np.inf)

    within_cutoff = r2 < (cutoff * cutoff)
    r2_clamped = np.maximum(r2, min_r2)

    inv_r2 = 1.0 / r2_clamped
    sr6 = (sigma * sigma * inv_r2) ** 3
    sr12 = sr6 * sr6

    # |F(r)| / r, so that force_vec = fmag_over_r * diff gives the force
    # directly without a separate normalisation step.
    fmag_over_r = 24.0 * epsilon * inv_r2 * (2.0 * sr12 - sr6)
    fmag_over_r = np.where(within_cutoff, fmag_over_r, 0.0)

    force = np.sum(fmag_over_r[:, :, None] * diff, axis=1)
    accel = force / mass[:, None]
    return [tuple(row) for row in accel]


def _transform_pure_python(cfg, entities):
    epsilon = cfg["epsilon"]
    sigma = cfg["sigma"]
    cutoff = cfg["cutoff"]
    min_r = cfg["min_r"]

    n = len(entities)
    accel = [[0.0, 0.0, 0.0] for _ in range(n)]

    for i in range(n):
        xi, yi, zi = entities[i][0], entities[i][1], entities[i][2]
        for j in range(i + 1, n):
            dx = xi - entities[j][0]
            dy = yi - entities[j][1]
            dz = zi - entities[j][2]
            r = math.sqrt(dx * dx + dy * dy + dz * dz)
            if r >= cutoff:
                continue
            r = max(r, min_r)

            sr6 = (sigma / r) ** 6
            sr12 = sr6 * sr6
            fmag_over_r = 24.0 * epsilon * (2.0 * sr12 - sr6) / (r * r)

            fx, fy, fz = fmag_over_r * dx, fmag_over_r * dy, fmag_over_r * dz
            mi, mj = entities[i][7], entities[j][7]

            accel[i][0] += fx / mi
            accel[i][1] += fy / mi
            accel[i][2] += fz / mi
            accel[j][0] -= fx / mj
            accel[j][1] -= fy / mj
            accel[j][2] -= fz / mj

    return [tuple(a) for a in accel]


def get_property_descriptions():
    return json.dumps(
        {
            "epsilon": "Depth of the Lennard-Jones potential well (interaction strength). Default 1.0",
            "sigma": "Length scale of a particle (roughly its diameter). Default 0.05",
            "cutoff": "Interaction cutoff distance, as a multiple of sigma. Default 2.5",
        }
    )
