/*
 * c_schwarzschild_geodesic — Schwarzschild null geodesic transform
 *                            implemented in C++ using Vari-Tensor
 *
 * Computes the same stable acceleration as the Rust schwarzschild_geodesic
 * element, but uses Vari-Tensor's Einstein summation to derive the angular
 * momentum manifestly from the 2-D Levi-Civita symbol:
 *
 *   L  = ε_{ab} x^a ẋ^b          (Vari-Tensor contraction)
 *   A  = −3·rs·L² / (2r⁴)
 *   aˢ = A · xˢ / r
 *
 * Only massless (mass == 0), non-fixed entities are affected.
 * Particles at or inside the Schwarzschild radius are skipped.
 *
 * Prerequisites
 * ─────────────
 *   varitensor.h must be present in this directory.  Run
 *       make varitensor.h
 *   to download it automatically, or place it here manually.
 *
 * Compile with C++23 (see makefile target "geodesic").
 */

extern "C" {
#include "physim.h"
}

#include "varitensor.h"

#include <cmath>
#include <cstring>
#include <cstdlib>
#include <new>

using namespace varitensor;

// ── Plugin registration constants (C linkage) ─────────────────────────────────

extern "C" {

const char* PLUGIN_ABI_INFO = "C";
const char* PLUGIN_ELEMENTS = "c_schwarzschild_geodesic";

static void* GLOBAL_BUS_TARGET = nullptr;

const char* get_plugin_abi_info() { return PLUGIN_ABI_INFO; }
const char* register_plugin()     { return PLUGIN_ELEMENTS; }
void set_callback_target(void* t) { GLOBAL_BUS_TARGET = t; }

} // extern "C"

// ── Element state ─────────────────────────────────────────────────────────────
//
// The 2-D Levi-Civita symbol and the Index objects used to address it are
// computed once at initialisation and stored here.  Reusing the same Index
// objects in every transform call ensures that Einstein contraction resolves
// correctly (the library matches indices by ID, not by name).

struct GeodesicState {
    double rs;
    Index  a, b;   // 2-D equatorial-plane coordinate indices
    Tensor eps;    // ε_{ab}: 2-D Levi-Civita symbol

    // Members are initialised in declaration order: rs, a, b, then eps.
    // eps is constructed after a and b so their IDs are already assigned.
    explicit GeodesicState(double rs_)
        : rs(rs_), a(2), b(2), eps(levi_civita_symbol({a, b})) {}
};

// ── Minimal JSON number parser ────────────────────────────────────────────────

static double json_f64(const char* json, const char* key, double fallback) {
    const char* p = std::strstr(json, key);
    if (!p) return fallback;
    p += std::strlen(key);
    while (*p == ' ' || *p == ':' || *p == '\t') ++p;
    char* end;
    double v = std::strtod(p, &end);
    return (end == p) ? fallback : v;
}

// ── C API functions ───────────────────────────────────────────────────────────

extern "C" {

void* c_schwarzschild_geodesic_init(const uint8_t* config, size_t len) {
    double rs = 0.1;
    if (config && len > 0) {
        // config is not guaranteed null-terminated — copy before parsing
        char* buf = static_cast<char*>(std::malloc(len + 1));
        if (buf) {
            std::memcpy(buf, config, len);
            buf[len] = '\0';
            rs = json_f64(buf, "\"rs\"", 0.1);
            std::free(buf);
        }
    }
    try {
        return new GeodesicState(rs);
    } catch (...) {
        return nullptr;
    }
}

void c_schwarzschild_geodesic_transform(
    const void* obj,
    const Entity* state, size_t /*state_len*/,
    Acceleration* acceleration, size_t acceleration_len)
{
    if (!obj) return;
    const auto& el = *static_cast<const GeodesicState*>(obj);
    const double rs = el.rs;

    for (size_t i = 0; i < acceleration_len; ++i) {
        // Only massless, non-fixed entities follow null geodesics
        if (state[i].mass != 0.0 || state[i].fixed) continue;

        const double x  = state[i].x,  y  = state[i].y;
        const double vx = state[i].vx, vy = state[i].vy;

        const double r2 = x*x + y*y;
        const double r  = std::sqrt(r2);

        // Skip at or inside the event horizon to prevent divergence
        if (r <= rs || r < 1e-10) continue;

        // Build contravariant position and velocity vectors.
        // Both use el.a / el.b — the same Index objects stored in el.eps —
        // so the Einstein contraction below identifies the right axes.
        Tensor pos{el.a}; pos[0] = x;  pos[1] = y;
        Tensor vel{el.b}; vel[0] = vx; vel[1] = vy;

        // Angular momentum:  L = ε_{ab} x^a ẋ^b
        // Repeated indices a, b are contracted automatically by Vari-Tensor.
        // Result: L = x·vy − y·vx (conserved along the null geodesic).
        const Tensor L_scalar = el.eps[el.a, el.b] * pos[el.a] * vel[el.b];
        const double L = static_cast<double>(L_scalar);

        // Stable radial acceleration — no 1/(1−rs/r) singularity:
        //   A ≡ r̈ − r·φ̇² = −3·rs·L² / (2r⁴)
        const double big_a = -3.0 * rs * L * L / (2.0 * r2 * r2);

        // Project back to Cartesian (φ̈ cross-terms cancel identically)
        acceleration[i].x += big_a * x / r;
        acceleration[i].y += big_a * y / r;
    }
}

void c_schwarzschild_geodesic_destroy(void* obj) {
    delete static_cast<GeodesicState*>(obj);
}

char* c_schwarzschild_geodesic_get_property_descriptions(void* /*obj*/, RustStringAllocFn alloc) {
    return alloc("{\"rs\": \"Schwarzschild radius rs = 2GM/c^2 of the central body (default 0.1)\"}");
}

void c_schwarzschild_geodesic_recv_message(void* /*obj*/, const CMessage* /*msg*/) {}

void c_schwarzschild_geodesic_post_configuration_messages(void* /*obj*/) {}

const TransformElementAPI* c_schwarzschild_geodesic_get_api() {
    static TransformElementAPI api = {
        .init                        = c_schwarzschild_geodesic_init,
        .transform                   = c_schwarzschild_geodesic_transform,
        .destroy                     = c_schwarzschild_geodesic_destroy,
        .get_property_descriptions   = c_schwarzschild_geodesic_get_property_descriptions,
        .recv_message                = c_schwarzschild_geodesic_recv_message,
        .post_configuration_messages = c_schwarzschild_geodesic_post_configuration_messages,
    };
    return &api;
}

ElementMetaFFI c_schwarzschild_geodesic_register(RustStringAllocFn alloc) {
    ElementMetaFFI meta;
    meta.kind    = Transform;
    meta.name    = alloc("c_schwarzschild_geodesic");
    meta.plugin  = alloc("c_geodesic_plugin");
    meta.version = alloc("0.1.0");
    meta.license = alloc("MPL-2.0");
    meta.author  = alloc("Joseph Briggs <jhbriggs23@gmail.com>");
    meta.blurb   = alloc("Schwarzschild null geodesic; L computed via Vari-Tensor Einstein summation");
    meta.repo    = alloc("https://github.com/jhb123/physim");
    return meta;
}

} // extern "C"
