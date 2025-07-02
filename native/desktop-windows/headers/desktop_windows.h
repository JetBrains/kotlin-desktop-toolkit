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

typedef enum NativeWindowSystemBackdropType {
  NativeWindowSystemBackdropType_Auto,
  NativeWindowSystemBackdropType_None,
  NativeWindowSystemBackdropType_Mica,
  NativeWindowSystemBackdropType_DesktopAcrylic,
  NativeWindowSystemBackdropType_MicaAlt,
} NativeWindowSystemBackdropType;

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

typedef struct NativePhysicalPoint {
  NativePhysicalPixels x;
  NativePhysicalPixels y;
} NativePhysicalPoint;

typedef struct NativeWindowScaleChangedEvent {
  struct NativePhysicalPoint new_origin;
  struct NativePhysicalSize new_size;
  float new_scale;
} NativeWindowScaleChangedEvent;

typedef enum NativeEvent_Tag {
  NativeEvent_WindowCloseRequest,
  NativeEvent_WindowDraw,
  NativeEvent_WindowScaleChanged,
} NativeEvent_Tag;

typedef struct NativeEvent {
  NativeEvent_Tag tag;
  union {
    struct {
      struct NativeWindowDrawEvent window_draw;
    };
    struct {
      struct NativeWindowScaleChangedEvent window_scale_changed;
    };
  };
} NativeEvent;

typedef bool (*NativeEventHandler)(NativeWindowId, const struct NativeEvent*);

typedef struct NativeApplicationCallbacks {
  NativeEventHandler event_handler;
} NativeApplicationCallbacks;

typedef NativeRustAllocatedRawPtr NativeAngleDevicePtr;

typedef struct NativeEglGetProcFuncData {
  void (*(*f)(NativeAngleDevicePtr ctx, NativeBorrowedStrPtr name))(void);
  NativeAngleDevicePtr ctx;
} NativeEglGetProcFuncData;

typedef NativeRustAllocatedRawPtr NativeWindowPtr;

typedef void (*NativeAngleDeviceDrawFun)(void);

typedef struct NativeAngleDeviceCallbacks {
  NativeAngleDeviceDrawFun draw_fun;
} NativeAngleDeviceCallbacks;

typedef float NativeLogicalPixels;

typedef struct NativeLogicalPoint {
  NativeLogicalPixels x;
  NativeLogicalPixels y;
} NativeLogicalPoint;

typedef struct NativeLogicalSize {
  NativeLogicalPixels width;
  NativeLogicalPixels height;
} NativeLogicalSize;

typedef struct NativeWindowParams {
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
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

struct NativeEglGetProcFuncData renderer_angle_get_egl_get_proc_func(NativeAngleDevicePtr angle_device_ptr);

NativeAngleDevicePtr renderer_angle_device_create(NativeWindowPtr window_ptr);

void renderer_angle_make_surface(NativeAngleDevicePtr angle_device_ptr,
                                 int32_t width,
                                 int32_t height);

void renderer_angle_draw(NativeAngleDevicePtr angle_device_ptr,
                         bool wait_for_vsync,
                         struct NativeAngleDeviceCallbacks callbacks);

void renderer_angle_drop(NativeAngleDevicePtr angle_device_ptr);

NativeWindowPtr window_create(NativeAppPtr app_ptr, struct NativeWindowParams params);

NativeWindowId window_get_window_id(NativeWindowPtr window_ptr);

void window_extend_content_into_titlebar(NativeWindowPtr window_ptr);

void window_apply_system_backdrop(NativeWindowPtr window_ptr,
                                  enum NativeWindowSystemBackdropType backdrop_type);

void window_show(NativeWindowPtr window_ptr);

void window_set_rect(NativeWindowPtr window_ptr,
                     struct NativePhysicalPoint origin,
                     struct NativePhysicalSize size);

void window_drop(NativeWindowPtr window_ptr);
