/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr_c_void;

typedef NativeRustAllocatedRawPtr_c_void NativeAppPtr;

typedef struct NativeApplicationConfig {

} NativeApplicationConfig;

typedef struct NativeApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  void (*on_display_configuration_change)(void);
} NativeApplicationCallbacks;

typedef uint32_t NativeWindowId;

typedef uintptr_t NativeKeyModifiersSet;

typedef uint16_t NativeKeyCode;

typedef const char *NativeGenericRawPtr_c_char;

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

typedef double NativeLogicalPixels;

typedef struct NativeLogicalPoint {
  NativeLogicalPixels x;
  NativeLogicalPixels y;
} NativeLogicalPoint;

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

typedef uint32_t NativeScreenId;

typedef struct NativeWindowScreenChangeEvent {
  NativeScreenId new_screen_id;
} NativeWindowScreenChangeEvent;

typedef struct NativeLogicalSize {
  NativeLogicalPixels width;
  NativeLogicalPixels height;
} NativeLogicalSize;

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

typedef struct NativeWindowCloseRequestEvent {

} NativeWindowCloseRequestEvent;

typedef struct NativeWindowFullScreenToggleEvent {
  bool is_full_screen;
} NativeWindowFullScreenToggleEvent;

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
      struct NativeWindowCloseRequestEvent window_close_request;
    };
    struct {
      struct NativeWindowFullScreenToggleEvent window_full_screen_toggle;
    };
  };
} NativeEvent;

typedef bool (*NativeEventHandler)(const struct NativeEvent*);

typedef struct NativeWindowParams {
  NativeEventHandler event_handler;
  uint32_t width;
  uint32_t height;
} NativeWindowParams;

NativeAppPtr application_init(const struct NativeApplicationConfig *_config,
                              struct NativeApplicationCallbacks callbacks);

void application_run_event_loop(NativeAppPtr app_ptr);

void application_stop_event_loop(NativeAppPtr app_ptr);

void application_shutdown(NativeAppPtr app_ptr);

NativeWindowId window_create(NativeAppPtr app_ptr, struct NativeWindowParams params);

void window_drop(NativeAppPtr app_ptr, NativeWindowId window_id);

struct NativeLogicalSize window_get_size(NativeAppPtr app_ptr, NativeWindowId window_id);
