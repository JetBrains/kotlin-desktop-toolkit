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

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeAppPtr;

typedef struct NativeApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  void (*on_display_configuration_change)(void);
} NativeApplicationCallbacks;

typedef uint32_t NativeScreenId;

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeRustAllocatedStrPtr;

typedef NativeRustAllocatedStrPtr NativeAutoDropStrPtr;

typedef double NativeLogicalPixels;

typedef struct NativeLogicalPoint {
  NativeLogicalPixels x;
  NativeLogicalPixels y;
} NativeLogicalPoint;

typedef struct NativeLogicalSize {
  NativeLogicalPixels width;
  NativeLogicalPixels height;
} NativeLogicalSize;

typedef struct NativeScreenInfo {
  NativeScreenId screen_id;
  bool is_primary;
  NativeAutoDropStrPtr name;
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
  double scale;
} NativeScreenInfo;

typedef uintptr_t NativeArraySize;

typedef struct NativeAutoDropArray_ScreenInfo {
  const struct NativeScreenInfo *ptr;
  NativeArraySize len;
} NativeAutoDropArray_ScreenInfo;

typedef struct NativeAutoDropArray_ScreenInfo NativeScreenInfoArray;

typedef uint32_t NativeWindowId;

typedef uintptr_t NativeKeyModifiersSet;

typedef uint16_t NativeKeyCode;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

typedef uint32_t NativeTimestamp;

typedef struct NativeKeyDownEvent {
  NativeKeyModifiersSet modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  bool is_repeat;
  NativeTimestamp timestamp;
} NativeKeyDownEvent;

typedef struct NativeKeyUpEvent {
  NativeKeyModifiersSet modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  NativeTimestamp timestamp;
} NativeKeyUpEvent;

typedef struct NativeModifiersChangedEvent {
  NativeKeyModifiersSet modifiers;
  NativeKeyCode code;
  NativeTimestamp timestamp;
} NativeModifiersChangedEvent;

typedef struct NativeMouseMovedEvent {
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseMovedEvent;

typedef uint32_t NativeMouseButton;

typedef struct NativeMouseDraggedEvent {
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseDraggedEvent;

typedef struct NativeMouseEnteredEvent {
  struct NativeLogicalPoint location_in_window;
} NativeMouseEnteredEvent;

typedef struct NativeMouseExitedEvent {
  struct NativeLogicalPoint location_in_window;
} NativeMouseExitedEvent;

typedef struct NativeMouseDownEvent {
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseDownEvent;

typedef struct NativeMouseUpEvent {
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseUpEvent;

typedef struct NativeScrollWheelEvent {
  NativeLogicalPixels scrolling_delta_x;
  NativeLogicalPixels scrolling_delta_y;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeScrollWheelEvent;

typedef struct NativeWindowScreenChangeEvent {
  NativeScreenId new_screen_id;
} NativeWindowScreenChangeEvent;

typedef struct NativeWindowResizeEvent {
  struct NativeLogicalSize size;
} NativeWindowResizeEvent;

typedef struct NativeWindowMoveEvent {
  struct NativeLogicalPoint origin;
} NativeWindowMoveEvent;

typedef struct NativeWindowFocusChangeEvent {
  bool is_key;
  bool is_main;
} NativeWindowFocusChangeEvent;

typedef struct NativeWindowFullScreenToggleEvent {
  bool is_full_screen;
} NativeWindowFullScreenToggleEvent;

typedef struct NativeWindowDrawEvent {
  uint8_t *buffer;
  uint32_t width;
  uint32_t height;
  uint32_t stride;
  double scale;
} NativeWindowDrawEvent;

typedef struct NativeWindowScaleChangedEvent {
  double new_scale;
} NativeWindowScaleChangedEvent;

typedef enum NativeEvent_Tag {
  NativeEvent_KeyDown,
  NativeEvent_KeyUp,
  NativeEvent_ModifiersChanged,
  NativeEvent_MouseMoved,
  NativeEvent_MouseDragged,
  NativeEvent_MouseEntered,
  NativeEvent_MouseExited,
  NativeEvent_MouseDown,
  NativeEvent_MouseUp,
  NativeEvent_ScrollWheel,
  NativeEvent_WindowScreenChange,
  NativeEvent_WindowResize,
  NativeEvent_WindowMove,
  NativeEvent_WindowFocusChange,
  NativeEvent_WindowCloseRequest,
  NativeEvent_WindowFullScreenToggle,
  NativeEvent_WindowDraw,
  NativeEvent_WindowScaleChanged,
} NativeEvent_Tag;

typedef struct NativeEvent {
  NativeEvent_Tag tag;
  union {
    struct {
      struct NativeKeyDownEvent key_down;
    };
    struct {
      struct NativeKeyUpEvent key_up;
    };
    struct {
      struct NativeModifiersChangedEvent modifiers_changed;
    };
    struct {
      struct NativeMouseMovedEvent mouse_moved;
    };
    struct {
      struct NativeMouseDraggedEvent mouse_dragged;
    };
    struct {
      struct NativeMouseEnteredEvent mouse_entered;
    };
    struct {
      struct NativeMouseExitedEvent mouse_exited;
    };
    struct {
      struct NativeMouseDownEvent mouse_down;
    };
    struct {
      struct NativeMouseUpEvent mouse_up;
    };
    struct {
      struct NativeScrollWheelEvent scroll_wheel;
    };
    struct {
      struct NativeWindowScreenChangeEvent window_screen_change;
    };
    struct {
      struct NativeWindowResizeEvent window_resize;
    };
    struct {
      struct NativeWindowMoveEvent window_move;
    };
    struct {
      struct NativeWindowFocusChangeEvent window_focus_change;
    };
    struct {
      struct NativeWindowFullScreenToggleEvent window_full_screen_toggle;
    };
    struct {
      struct NativeWindowDrawEvent window_draw;
    };
    struct {
      struct NativeWindowScaleChangedEvent window_scale_changed;
    };
  };
} NativeEvent;

typedef bool (*NativeEventHandler)(const struct NativeEvent*);

typedef struct NativeWindowParams {
  uint32_t width;
  uint32_t height;
} NativeWindowParams;

typedef struct NativeExceptionsArray {
  const NativeRustAllocatedStrPtr *items;
  NativeArraySize count;
} NativeExceptionsArray;

typedef struct NativeLoggerConfiguration {
  NativeBorrowedStrPtr file_path;
  enum NativeLogLevel console_level;
  enum NativeLogLevel file_level;
} NativeLoggerConfiguration;

NativeAppPtr application_init(struct NativeApplicationCallbacks callbacks);

void application_run_event_loop(NativeAppPtr app_ptr);

void application_stop_event_loop(NativeAppPtr app_ptr);

void application_shutdown(NativeAppPtr app_ptr);

NativeScreenInfoArray screen_list(NativeAppPtr app_ptr);

void screen_list_drop(NativeScreenInfoArray arr);

NativeWindowId window_create(NativeAppPtr app_ptr,
                             NativeEventHandler event_handler,
                             struct NativeWindowParams params);

void window_drop(NativeAppPtr app_ptr, NativeWindowId window_id);

struct NativeLogicalSize window_get_size(NativeAppPtr app_ptr, NativeWindowId window_id);

struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);
