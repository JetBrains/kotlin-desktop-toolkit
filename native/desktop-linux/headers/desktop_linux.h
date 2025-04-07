/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeAppPtr;

typedef struct NativeApplicationConfig {

} NativeApplicationConfig;

typedef struct NativeApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  void (*on_display_configuration_change)(void);
} NativeApplicationCallbacks;

NativeAppPtr application_init(const struct NativeApplicationConfig *_config,
                              struct NativeApplicationCallbacks callbacks);

void application_run_event_loop(NativeAppPtr app_ptr);

void application_stop_event_loop(NativeAppPtr app_ptr);

void application_shutdown(NativeAppPtr app_ptr);
