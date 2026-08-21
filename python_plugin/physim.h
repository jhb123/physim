#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum ElementKind {
  Initialiser,
  Transform,
  Render,
  Synth,
  Transmute,
  Integrator,
} ElementKind;

typedef enum MessageOrigin {
  Rust = 0,
  C = 1,
} MessageOrigin;

typedef enum MessagePriority {
  Background,
  Low,
  Normal,
  High,
  RealTime,
  Critical,
} MessagePriority;

typedef struct CMessage {
  enum MessagePriority priority;
  const char *topic;
  const char *message;
  uintptr_t sender_id;
  enum MessageOrigin origin;
} CMessage;

typedef struct Entity {
  double x;
  double y;
  double z;
  double vx;
  double vy;
  double vz;
  double radius;
  double mass;
  uintptr_t id;
  bool fixed;
} Entity;

typedef struct Acceleration {
  double x;
  double y;
  double z;
} Acceleration;

typedef char *(*RustStringAllocFn)(const char*);

typedef struct TransformElementAPI {
  void *(*init)(const uint8_t*, uintptr_t);
  void (*transform)(const void*, const struct Entity*, uintptr_t, struct Acceleration*, uintptr_t);
  void (*destroy)(void*);
  char *(*get_property_descriptions)(void*, RustStringAllocFn);
  void (*recv_message)(void *obj, const struct CMessage *msg);
  void (*post_configuration_messages)(void *obj);
} TransformElementAPI;

/**
 * FFI-compatible version
 */
typedef struct ElementMetaFFI {
  enum ElementKind kind;
  char *name;
  char *plugin;
  char *version;
  char *license;
  char *author;
  char *blurb;
  char *repo;
} ElementMetaFFI;

void post_bus_callback(void *target, struct CMessage message);

/**
 * Host allocator functions to pass to plugins
 * # Safety
 *  Consult [`CStr::from_ptr`]
 */
char *host_alloc_string(const char *s);

/**
 * # Safety
 *  Consult [`CString::from_raw`]
 */
void host_free_string(char *s);
