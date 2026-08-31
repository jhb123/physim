/*
 This is a physim plugin, written like c_plugin, except the physics itself
 lives in Python (gas.py, next to this file) instead of in C. physim only
 ever dlopen()s compiled libraries with a specific C ABI - it can't load a
 .py file directly - so this file's job is just to embed a CPython
 interpreter and forward calls into gas.py.

 This plugin registers one element, `gas`, a transform element which
 simulates a gas: every pair of entities interacts through a Lennard-Jones
 potential (short range repulsion, weak attraction just beyond that, no
 force past a cutoff). See gas.py for the actual force calculation.

 physim.h is generated with the cbindgen tool. To compile this plugin you
 need to link against lphysim_core, and a Python 3 installation (see the
 makefile, which shells out to python3-config for the right flags).
*/

#define PY_SSIZE_T_CLEAN
#include <Python.h>

#include <dlfcn.h>
#include <libgen.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "physim.h"

/*****************************************************************************
 * These methods are boiler plate for registering elements in your plugin.
 *****************************************************************************/

/* For non-Rust plugins, PLUGIN_ABI_INFO has to be "C". physim will attempt
   to validate that the plugin was compiled with the same version of rust
   when the plugin is loaded, but in the case of C (and, as here, C
   embedding Python), all bets are off. */
const char *PLUGIN_ABI_INFO = "C";

/* PLUGIN_ELEMENTS is a comma separated list of elements. physim will parse
   this to know what to look for in the plugin library. This tells physim
   to look for gas_get_api */
const char *PLUGIN_ELEMENTS = "gas";

/* Global bus target for passing messages onto the message bus. physim will
   set this during the pipeline's life cycle using set_callback_target */
static void *GLOBAL_BUS_TARGET = NULL;

const char *get_plugin_abi_info(void) { return PLUGIN_ABI_INFO; }

const char *register_plugin(void) { return PLUGIN_ELEMENTS; }

void set_callback_target(void *target) {
    if (target == NULL) {
        fprintf(stderr, "Error: callback target is null\n");
        abort();
    }
    GLOBAL_BUS_TARGET = target;
}

/*****************************************************************************
 * Embedding CPython.
 *
 * Py_Initialize() is only safe to call once per process, and after it runs
 * on the thread that calls it, that thread implicitly holds the GIL. physim
 * calls into this plugin from a thread CPython doesn't know about, so
 * straight after initialising we release the GIL with PyEval_SaveThread()
 * and go back to acquiring/releasing it around every call with
 * PyGILState_Ensure()/PyGILState_Release(), which is the documented pattern
 * for embedding in a foreign host application.
 *****************************************************************************/

static int PYTHON_READY = 0;
static PyObject *GAS_MODULE = NULL;

/* physim can load a plugin from arbitrary directories (the binary's own
   directory, or anywhere listed in PHYSIM_PLUGIN_DIR), so we can't assume
   gas.py is reachable from the process's cwd or default sys.path. Instead
   we ask the dynamic loader where *this* shared library was mapped from,
   and add that directory to sys.path before importing gas. */
static void add_plugin_dir_to_syspath(void) {
    Dl_info info;
    if (dladdr((void *)add_plugin_dir_to_syspath, &info) == 0 || info.dli_fname == NULL) {
        fprintf(stderr, "gas plugin: could not resolve own library path, "
                         "'import gas' may fail\n");
        return;
    }

    char path_copy[4096];
    strncpy(path_copy, info.dli_fname, sizeof(path_copy) - 1);
    path_copy[sizeof(path_copy) - 1] = '\0';
    const char *dir = dirname(path_copy);

    PyObject *sys_path = PySys_GetObject("path"); /* borrowed reference */
    PyObject *dir_obj = PyUnicode_FromString(dir);
    if (sys_path != NULL && dir_obj != NULL) {
        PyList_Insert(sys_path, 0, dir_obj);
    }
    Py_XDECREF(dir_obj);
}

static void ensure_python_initialised(void) {
    if (PYTHON_READY) {
        return;
    }
    PYTHON_READY = 1; /* set before init so we don't retry forever on failure */

    Py_Initialize();
    add_plugin_dir_to_syspath();

    GAS_MODULE = PyImport_ImportModule("gas");
    if (GAS_MODULE == NULL) {
        fprintf(stderr, "gas plugin: failed to import gas.py\n");
        PyErr_Print();
    }

    /* Release the GIL now that setup is done; every call site below
       re-acquires it with PyGILState_Ensure. */
    PyEval_SaveThread();
}

/*****************************************************************************
 * These methods give your element behaviour.
 *****************************************************************************/

/* GasTransform maintains the element's state: a reference to the config
   dict gas.init() built from the pipeline's JSON config for this element. */
typedef struct {
    PyObject *py_config;
} GasTransform;

void *gas_init(const uint8_t *config, size_t len) {
    ensure_python_initialised();
    if (GAS_MODULE == NULL) {
        return NULL;
    }

    GasTransform *el = (GasTransform *)malloc(sizeof(GasTransform));
    if (el == NULL) {
        return NULL;
    }

    PyGILState_STATE gstate = PyGILState_Ensure();

    PyObject *config_str = (config != NULL)
                                ? PyUnicode_FromStringAndSize((const char *)config, (Py_ssize_t)len)
                                : PyUnicode_FromString("{}");

    PyObject *cfg = PyObject_CallMethod(GAS_MODULE, "init", "O", config_str);
    Py_XDECREF(config_str);

    if (cfg == NULL) {
        PyErr_Print();
        PyGILState_Release(gstate);
        free(el);
        return NULL;
    }

    el->py_config = cfg; /* owned reference, released in gas_destroy */
    PyGILState_Release(gstate);
    return (void *)el;
}

/* Transform elements call this function every time they need to update the
   state of the system. state and acceleration are the same length. */
void gas_transform(const void *obj, const Entity *state, size_t state_len,
                    Acceleration *acceleration, size_t acceleration_len) {
    if (obj == NULL || GAS_MODULE == NULL) {
        return;
    }
    const GasTransform *el = (const GasTransform *)obj;

    PyGILState_STATE gstate = PyGILState_Ensure();

    PyObject *entities = PyList_New((Py_ssize_t)state_len);
    if (entities == NULL) {
        PyErr_Print();
        PyGILState_Release(gstate);
        return;
    }
    for (size_t i = 0; i < state_len; i++) {
        const Entity *e = &state[i];
        PyObject *tuple = Py_BuildValue("(dddddddd)", e->x, e->y, e->z, e->vx, e->vy,
                                         e->vz, e->radius, e->mass);
        PyList_SET_ITEM(entities, (Py_ssize_t)i, tuple); /* steals reference */
    }

    PyObject *result =
        PyObject_CallMethod(GAS_MODULE, "transform", "OO", el->py_config, entities);
    Py_DECREF(entities);

    if (result == NULL) {
        PyErr_Print();
        PyGILState_Release(gstate);
        return;
    }

    Py_ssize_t returned = PyList_Size(result);
    if (returned != (Py_ssize_t)acceleration_len) {
        fprintf(stderr, "gas plugin: expected %zu accelerations, got %zd\n",
                acceleration_len, returned);
        Py_DECREF(result);
        PyGILState_Release(gstate);
        return;
    }

    for (size_t i = 0; i < acceleration_len; i++) {
        PyObject *row = PyList_GET_ITEM(result, (Py_ssize_t)i); /* borrowed */
        acceleration[i].x += PyFloat_AsDouble(PyTuple_GET_ITEM(row, 0));
        acceleration[i].y += PyFloat_AsDouble(PyTuple_GET_ITEM(row, 1));
        acceleration[i].z += PyFloat_AsDouble(PyTuple_GET_ITEM(row, 2));
    }
    Py_DECREF(result);
    PyGILState_Release(gstate);

    if (GLOBAL_BUS_TARGET != NULL) {
        CMessage msg;
        msg.topic = "gasTransform";
        msg.message = "transformed";
        msg.priority = Normal;
        msg.sender_id = (size_t)obj;
        msg.origin = C;
        post_bus_callback(GLOBAL_BUS_TARGET, msg);
    }
}

/* This is called by physim when it is finished using the element. */
void gas_destroy(void *obj) {
    if (obj == NULL) {
        return;
    }
    GasTransform *el = (GasTransform *)obj;
    if (GAS_MODULE != NULL) {
        PyGILState_STATE gstate = PyGILState_Ensure();
        Py_XDECREF(el->py_config);
        PyGILState_Release(gstate);
    }
    free(el);
}

/* Documentation about the element for users of physcan. Doesn't affect the
   element when it runs in a simulation. */
char *gas_get_property_descriptions(void *obj, RustStringAllocFn alloc) {
    if (obj == NULL || GAS_MODULE == NULL) {
        return NULL;
    }

    PyGILState_STATE gstate = PyGILState_Ensure();
    PyObject *result = PyObject_CallMethod(GAS_MODULE, "get_property_descriptions", NULL);
    if (result == NULL) {
        PyErr_Print();
        PyGILState_Release(gstate);
        return NULL;
    }

    const char *json = PyUnicode_AsUTF8(result);
    char *out = (json != NULL) ? alloc(json) : NULL;
    Py_DECREF(result);
    PyGILState_Release(gstate);
    return out;
}

/* The message bus is a way of broadcasting information throughout the
   pipeline so that elements can talk to each other. gas doesn't need to
   react to messages, so this is a no-op. */
void gas_recv_message(void *obj, const struct CMessage *msg) {
    if (obj == NULL) {
        return;
    }
    (void)msg; // Unused
}

/* After the initialisation of an element, it gets 1 chance to broadcast
   some information to the rest of the pipeline before the simulation
   starts. gas doesn't have anything to announce. */
void gas_post_configuration_messages(void *obj) {
    if (obj == NULL) {
        return;
    }
}

/* This wires up the element and makes it an "object" */
const TransformElementAPI *gas_get_api(void) {
    static TransformElementAPI api = {
        .init = gas_init,
        .transform = gas_transform,
        .destroy = gas_destroy,
        .get_property_descriptions = gas_get_property_descriptions,
        .recv_message = gas_recv_message,
        .post_configuration_messages = gas_post_configuration_messages,
    };
    return &api;
}

/* Metadata shown to users of physcan, and used to register the element in
   physim's pool of registered elements. */
ElementMetaFFI gas_register(RustStringAllocFn alloc) {
    ElementMetaFFI meta;
    meta.kind = Transform; /* the only kind of element pluggable over the C ABI */
    meta.name = alloc("gas");
    meta.plugin = alloc("python_plugin");
    meta.version = alloc("0.1.0");
    meta.license = alloc("MIT");
    meta.author = alloc("Joe O'Donnell <joe.odonnell27@gmail.com>");
    meta.blurb = alloc("Lennard-Jones gas simulation, implemented in Python");
    meta.repo = alloc("https://github.com/jhb123/physim");
    return meta;
}
