#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Forward declarations for the types you'll need to define based on physim_core
// These are placeholder definitions - adjust based on your actual C headers
typedef struct Entity Entity;
typedef struct Acceleration Acceleration;
typedef struct Message Message;
typedef struct CMessage CMessage;

typedef enum {
    MESSAGE_PRIORITY_LOW = 0,
    MESSAGE_PRIORITY_NORMAL = 1,
    MESSAGE_PRIORITY_HIGH = 2,
} MessagePriority;

typedef enum {
    ELEMENT_KIND_TRANSFORM = 1,
} ElementKind;

typedef struct {
    ElementKind kind;
    const char* name;
    const char* plugin;
    const char* version;
    const char* license;
    const char* author;
    const char* blurb;
    const char* repo;
} ElementMeta;

// Function pointer types
typedef void* (*InitFn)(const uint8_t* config, size_t len);
typedef void (*TransformFn)(const void* obj, const Entity* state, size_t state_len,
                            Acceleration* acceleration, size_t acceleration_len);
typedef void (*DestroyFn)(void* obj);
typedef char* (*GetPropertyDescriptionsFn)(void* obj);
typedef void (*RecvMessageFn)(void* obj, void* msg);
typedef void (*PostConfigurationMessagesFn)(void* obj);

// TransformElementAPI structure
typedef struct {
    InitFn init;
    TransformFn transform;
    DestroyFn destroy;
    GetPropertyDescriptionsFn get_property_descriptions;
    RecvMessageFn recv_message;
    PostConfigurationMessagesFn post_configuration_messages;
} TransformElementAPI;

// Global bus target for message passing
static void* GLOBAL_BUS_TARGET = NULL;

// External callback function (provided by the host)
extern void physim_core_messages_callback(void* target, CMessage msg);

// The DebugTransform structure (empty in this case)
typedef struct {
    // No fields needed for cdebug transform
    int dummy; // C doesn't allow empty structs
} DebugTransform;

// Plugin ABI info
const char* PLUGIN_ABI_INFO = "rustc:1.86.0-nightly|target:aarch64-apple-darwin";
const char* ELEMENT_NAME = "cdebug";


char* get_plugin_abi_info(void) {
    return PLUGIN_ABI_INFO;
}

char* register_plugin(void) {
    return ELEMENT_NAME;
}

void set_callback_target(void* target) {
    if (target == NULL) {
        fprintf(stderr, "Error: callback target is null\n");
        abort();
    }
    GLOBAL_BUS_TARGET = target;
}

// Debug element initialization
void* cdebug_init(const uint8_t* config, size_t len) {
    if (config == NULL) {
        return NULL;
    }
    
    // For this simple cdebug element, we don't parse the config
    // In a real implementation, you'd parse the JSON config here
    
    DebugTransform* transform = (DebugTransform*)malloc(sizeof(DebugTransform));
    if (transform == NULL) {
        return NULL;
    }
    
    transform->dummy = 0;
    return (void*)transform;
}

// Debug transform function
void cdebug_transform(const void* obj, const Entity* state, size_t state_len,
                     Acceleration* acceleration, size_t acceleration_len) {
    const DebugTransform* el = (const DebugTransform*)obj;
    (void)el; // Unused
    (void)state; // Unused
    (void)state_len; // Unused
    
    // Log transform (you'll need to implement logging based on your system)
    printf("cDebug transform\n");
    
    // The acceleration array is already initialized, we just pass through
    // In the Rust version, it adds default acceleration (which is zero)
    for (size_t i = 0; i < acceleration_len; i++) {
        // Add default acceleration (no-op for cdebug)
        // acceleration[i] += default_acceleration;
    }
    
    // Post message to bus
    if (GLOBAL_BUS_TARGET != NULL) {
        // Create message - you'll need to implement CMessage creation
        // based on your actual message structure
        /*
        Message msg;
        msg.topic = "debugplugin";
        msg.message = "transformed";
        msg.priority = MESSAGE_PRIORITY_LOW;
        msg.sender_id = (size_t)obj;
        
        CMessage cmsg = message_to_c_message(&msg);
        physim_core_messages_callback(GLOBAL_BUS_TARGET, cmsg);
        */
    }
}

// Destroy function
void cdebug_destroy(void* obj) {
    if (obj == NULL) {
        return;
    }
    free(obj);
}

// Get property descriptions
char* cdebug_get_property_descriptions(void* obj) {
    if (obj == NULL) {
        return NULL;
    }
    
    // Return empty JSON object since cdebug has no properties
    return strdup("{}");
}

// Receive message
void cdebug_recv_message(void* obj, void* msg) {
    if (obj == NULL) {
        return;
    }
    
    // Debug element doesn't process messages
    // In a real implementation, you'd handle incoming messages here
}

// Post configuration messages
void cdebug_post_configuration_messages(void* obj) {
    if (obj == NULL) {
        return;
    }
    
    // Debug element doesn't post configuration messages
}

// Get the TransformElementAPI
const TransformElementAPI* cdebug_get_api(void) {
    static TransformElementAPI api = {
        .init = cdebug_init,
        .transform = cdebug_transform,
        .destroy = cdebug_destroy,
        .get_property_descriptions = cdebug_get_property_descriptions,
        .recv_message = cdebug_recv_message,
        .post_configuration_messages = cdebug_post_configuration_messages,
    };
    return &api;
}

typedef char* (*AllocStringFn)(const char*);

ElementMeta cdebug_register(AllocStringFn alloc) {
    ElementMeta meta;
    meta.kind = ELEMENT_KIND_TRANSFORM;
    meta.name = alloc("cdebug");
    meta.plugin = alloc("cplugin");
    meta.version = alloc("1.0.0");
    meta.license = alloc("MIT");
    meta.author = alloc("Joseph Briggs <jhbriggs23@gmail.com>");
    meta.blurb = alloc("Example of a C plugin");
    meta.repo = alloc("https://github.com/jhb123/physim");
    return meta;
}
