/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef enum NativeLogLevel {
  NativeLogLevel_Off,
  NativeLogLevel_Error,
  NativeLogLevel_Warn,
  NativeLogLevel_Info,
  NativeLogLevel_Debug,
  NativeLogLevel_Trace,
} NativeLogLevel;

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeRustAllocatedStrPtr;

typedef uintptr_t NativeArraySize;

typedef struct NativeExceptionsArray {
  const NativeRustAllocatedStrPtr *items;
  NativeArraySize count;
} NativeExceptionsArray;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

typedef struct NativeLoggerConfiguration {
  NativeBorrowedStrPtr file_path;
  enum NativeLogLevel console_level;
  enum NativeLogLevel file_level;
} NativeLoggerConfiguration;

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeAppPtr;

typedef intptr_t NativeWindowId;

typedef int32_t NativePhysicalPixels;

typedef struct NativePhysicalSize {
  NativePhysicalPixels width;
  NativePhysicalPixels height;
} NativePhysicalSize;

typedef struct NativeWindowDrawEvent {
  struct NativePhysicalSize physical_size;
  float scale;
} NativeWindowDrawEvent;

typedef enum NativeEvent_Tag {
  NativeEvent_WindowCloseRequest,
  NativeEvent_WindowDraw,
} NativeEvent_Tag;

typedef struct NativeEvent {
  NativeEvent_Tag tag;
  union {
    struct {
      struct NativeWindowDrawEvent window_draw;
    };
  };
} NativeEvent;

typedef bool (*NativeEventHandler)(NativeWindowId, const struct NativeEvent*);

typedef struct NativeApplicationCallbacks {
  NativeEventHandler event_handler;
} NativeApplicationCallbacks;

typedef NativeRustAllocatedRawPtr NativeWindowPtr;

typedef struct NativePhysicalPoint {
  NativePhysicalPixels x;
  NativePhysicalPixels y;
} NativePhysicalPoint;

typedef struct NativeWindowParams {
  struct NativePhysicalPoint origin;
  struct NativePhysicalSize size;
  NativeBorrowedStrPtr title;
  bool is_resizable;
  bool is_closable;
  bool is_minimizable;
} NativeWindowParams;

struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);

void logger_output_debug_string(NativeBorrowedStrPtr message);

NativeAppPtr application_init(struct NativeApplicationCallbacks callbacks);

void application_run_event_loop(NativeAppPtr app_ptr);

void application_stop_event_loop(NativeAppPtr app_ptr);

NativeWindowPtr window_create(NativeAppPtr app_ptr, struct NativeWindowParams params);

NativeWindowId window_get_window_id(NativeWindowPtr window_ptr);

void window_show(NativeWindowPtr window_ptr);

void window_drop(NativeWindowPtr window_ptr);
