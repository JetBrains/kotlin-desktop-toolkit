/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef enum NativeFontAntialiasing {
  NativeFontAntialiasing_None,
  NativeFontAntialiasing_Grayscale,
  NativeFontAntialiasing_Rgba,
} NativeFontAntialiasing;

typedef enum NativeFontHinting {
  NativeFontHinting_None,
  NativeFontHinting_Slight,
  NativeFontHinting_Medium,
  NativeFontHinting_Full,
} NativeFontHinting;

typedef enum NativeFontRgbaOrder {
  NativeFontRgbaOrder_Rgb,
  NativeFontRgbaOrder_Bgr,
  NativeFontRgbaOrder_Vrgb,
  NativeFontRgbaOrder_Vbgr,
} NativeFontRgbaOrder;

typedef enum NativeLogLevel {
  NativeLogLevel_Off,
  NativeLogLevel_Error,
  NativeLogLevel_Warn,
  NativeLogLevel_Info,
  NativeLogLevel_Debug,
  NativeLogLevel_Trace,
} NativeLogLevel;

typedef enum NativePointerShape {
  /**
   * The platform-dependent default cursor. Often rendered as arrow.
   */
  NativePointerShape_Default,
  /**
   * A context menu is available for the object under the cursor. Often
   * rendered as an arrow with a small menu-like graphic next to it.
   */
  NativePointerShape_ContextMenu,
  /**
   * Help is available for the object under the cursor. Often rendered as a
   * question mark or a balloon.
   */
  NativePointerShape_Help,
  /**
   * The cursor is a pointer that indicates a link. Often rendered as the
   * backside of a hand with the index finger extended.
   */
  NativePointerShape_Pointer,
  /**
   * A progress indicator. The program is performing some processing, but is
   * different from [`CursorIcon::Wait`] in that the user may still interact
   * with the program.
   */
  NativePointerShape_Progress,
  /**
   * Indicates that the program is busy and the user should wait. Often
   * rendered as a watch or hourglass.
   */
  NativePointerShape_Wait,
  /**
   * Indicates that a cell or set of cells may be selected. Often rendered as
   * a thick plus-sign with a dot in the middle.
   */
  NativePointerShape_Cell,
  /**
   * A simple crosshair (e.g., short line segments resembling a "+" sign).
   * Often used to indicate a two dimensional bitmap selection mode.
   */
  NativePointerShape_Crosshair,
  /**
   * Indicates text that may be selected. Often rendered as an I-beam.
   */
  NativePointerShape_Text,
  /**
   * Indicates vertical-text that may be selected. Often rendered as a
   * horizontal I-beam.
   */
  NativePointerShape_VerticalText,
  /**
   * Indicates an alias of/shortcut to something is to be created. Often
   * rendered as an arrow with a small curved arrow next to it.
   */
  NativePointerShape_Alias,
  /**
   * Indicates something is to be copied. Often rendered as an arrow with a
   * small plus sign next to it.
   */
  NativePointerShape_Copy,
  /**
   * Indicates something is to be moved.
   */
  NativePointerShape_Move,
  /**
   * Indicates that the dragged item cannot be dropped at the current cursor
   * location. Often rendered as a hand or pointer with a small circle with a
   * line through it.
   */
  NativePointerShape_NoDrop,
  /**
   * Indicates that the requested action will not be carried out. Often
   * rendered as a circle with a line through it.
   */
  NativePointerShape_NotAllowed,
  /**
   * Indicates that something can be grabbed (dragged to be moved). Often
   * rendered as the backside of an open hand.
   */
  NativePointerShape_Grab,
  /**
   * Indicates that something is being grabbed (dragged to be moved). Often
   * rendered as the backside of a hand with fingers closed mostly out of
   * view.
   */
  NativePointerShape_Grabbing,
  /**
   * The east border to be moved.
   */
  NativePointerShape_EResize,
  /**
   * The north border to be moved.
   */
  NativePointerShape_NResize,
  /**
   * The north-east corner to be moved.
   */
  NativePointerShape_NeResize,
  /**
   * The north-west corner to be moved.
   */
  NativePointerShape_NwResize,
  /**
   * The south border to be moved.
   */
  NativePointerShape_SResize,
  /**
   * The south-east corner to be moved.
   */
  NativePointerShape_SeResize,
  /**
   * The south-west corner to be moved.
   */
  NativePointerShape_SwResize,
  /**
   * The west border to be moved.
   */
  NativePointerShape_WResize,
  /**
   * The east and west borders to be moved.
   */
  NativePointerShape_EwResize,
  /**
   * The south and north borders to be moved.
   */
  NativePointerShape_NsResize,
  /**
   * The north-east and south-west corners to be moved.
   */
  NativePointerShape_NeswResize,
  /**
   * The north-west and south-east corners to be moved.
   */
  NativePointerShape_NwseResize,
  /**
   * Indicates that the item/column can be resized horizontally. Often
   * rendered as arrows pointing left and right with a vertical bar
   * separating them.
   */
  NativePointerShape_ColResize,
  /**
   * Indicates that the item/row can be resized vertically. Often rendered as
   * arrows pointing up and down with a horizontal bar separating them.
   */
  NativePointerShape_RowResize,
  /**
   * Indicates that the something can be scrolled in any direction. Often
   * rendered as arrows pointing up, down, left, and right with a dot in the
   * middle.
   */
  NativePointerShape_AllScroll,
  /**
   * Indicates that something can be zoomed in. Often rendered as a
   * magnifying glass with a "+" in the center of the glass.
   */
  NativePointerShape_ZoomIn,
  /**
   * Indicates that something can be zoomed in. Often rendered as a
   * magnifying glass with a "-" in the center of the glass.
   */
  NativePointerShape_ZoomOut,
} NativePointerShape;

enum NativeWindowButtonType {
  NativeWindowButtonType_AppMenu,
  NativeWindowButtonType_Icon,
  NativeWindowButtonType_Spacer,
  NativeWindowButtonType_Minimize,
  NativeWindowButtonType_Maximize,
  NativeWindowButtonType_Close,
};
typedef int32_t NativeWindowButtonType;

typedef enum NativeWindowResizeEdge {
  /**
   * Nothing is being dragged.
   */
  NativeWindowResizeEdge_None,
  /**
   * The top edge is being dragged.
   */
  NativeWindowResizeEdge_Top,
  /**
   * The bottom edge is being dragged.
   */
  NativeWindowResizeEdge_Bottom,
  /**
   * The left edge is being dragged.
   */
  NativeWindowResizeEdge_Left,
  /**
   * The top left corner is being dragged.
   */
  NativeWindowResizeEdge_TopLeft,
  /**
   * The bottom left corner is being dragged.
   */
  NativeWindowResizeEdge_BottomLeft,
  /**
   * The right edge is being dragged.
   */
  NativeWindowResizeEdge_Right,
  /**
   * The top right corner is being dragged.
   */
  NativeWindowResizeEdge_TopRight,
  /**
   * The bottom right corner is being dragged.
   */
  NativeWindowResizeEdge_BottomRight,
} NativeWindowResizeEdge;

typedef enum NativeXdgDesktopColorScheme {
  /**
   * No preference
   */
  NativeXdgDesktopColorScheme_NoPreference,
  /**
   * Prefers dark appearance
   */
  NativeXdgDesktopColorScheme_PreferDark,
  /**
   * Prefers light appearance
   */
  NativeXdgDesktopColorScheme_PreferLight,
} NativeXdgDesktopColorScheme;

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeAppPtr;

typedef uintptr_t NativeArraySize;

typedef struct NativeAutoDropArray_WindowButtonType {
  const NativeWindowButtonType *ptr;
  NativeArraySize len;
} NativeAutoDropArray_WindowButtonType;

typedef struct NativeTitlebarButtonLayout {
  struct NativeAutoDropArray_WindowButtonType left_side;
  struct NativeAutoDropArray_WindowButtonType right_side;
} NativeTitlebarButtonLayout;

typedef struct NativeColor {
  double red;
  double green;
  double blue;
  double alpha;
} NativeColor;

typedef enum NativeXdgDesktopSetting_Tag {
  NativeXdgDesktopSetting_TitlebarLayout,
  NativeXdgDesktopSetting_DoubleClickIntervalMs,
  NativeXdgDesktopSetting_ColorScheme,
  NativeXdgDesktopSetting_AccentColor,
  NativeXdgDesktopSetting_FontAntialiasing,
  NativeXdgDesktopSetting_FontHinting,
  NativeXdgDesktopSetting_FontRgbaOrder,
  NativeXdgDesktopSetting_CursorBlink,
  /**
   * Length of the cursor blink cycle, in milliseconds.
   */
  NativeXdgDesktopSetting_CursorBlinkTimeMs,
  /**
   * Time after which the cursor stops blinking.
   */
  NativeXdgDesktopSetting_CursorBlinkTimeoutMs,
  NativeXdgDesktopSetting_OverlayScrolling,
  NativeXdgDesktopSetting_AudibleBell,
} NativeXdgDesktopSetting_Tag;

typedef struct NativeXdgDesktopSetting {
  NativeXdgDesktopSetting_Tag tag;
  union {
    struct {
      struct NativeTitlebarButtonLayout titlebar_layout;
    };
    struct {
      int32_t double_click_interval_ms;
    };
    struct {
      enum NativeXdgDesktopColorScheme color_scheme;
    };
    struct {
      struct NativeColor accent_color;
    };
    struct {
      enum NativeFontAntialiasing font_antialiasing;
    };
    struct {
      enum NativeFontHinting font_hinting;
    };
    struct {
      enum NativeFontRgbaOrder font_rgba_order;
    };
    struct {
      bool cursor_blink;
    };
    struct {
      int32_t cursor_blink_time_ms;
    };
    struct {
      int32_t cursor_blink_timeout_ms;
    };
    struct {
      bool overlay_scrolling;
    };
    struct {
      bool audible_bell;
    };
  };
} NativeXdgDesktopSetting;

typedef struct NativeApplicationCallbacks {
  void (*on_application_started)(void);
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  void (*on_display_configuration_change)(void);
  void (*on_xdg_desktop_settings_change)(struct NativeXdgDesktopSetting);
} NativeApplicationCallbacks;

typedef NativeGenericRawPtr_c_void NativeBorrowedOpaquePtr;

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

typedef struct NativeGetEglProcFuncData {
  void (*(*f)(NativeBorrowedOpaquePtr ctx, NativeBorrowedStrPtr name))(void);
  NativeBorrowedOpaquePtr ctx;
} NativeGetEglProcFuncData;

typedef uint32_t NativeScreenId;

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
  int32_t maximum_frames_per_second;
} NativeScreenInfo;

typedef struct NativeAutoDropArray_ScreenInfo {
  const struct NativeScreenInfo *ptr;
  NativeArraySize len;
} NativeAutoDropArray_ScreenInfo;

typedef struct NativeAutoDropArray_ScreenInfo NativeScreenInfoArray;

typedef uint32_t NativeWindowId;

typedef struct NativeKeyModifiers {
  /**
   * The "control" key
   */
  bool ctrl;
  /**
   * The "alt" key
   */
  bool alt;
  /**
   * The "shift" key
   */
  bool shift;
  /**
   * The "Caps lock" key
   */
  bool caps_lock;
  /**
   * The "logo" key
   *
   * Also known as the "windows" or "super" key on a keyboard.
   */
  bool logo;
  /**
   * The "Num lock" key
   */
  bool num_lock;
} NativeKeyModifiers;

typedef uint32_t NativeKeyCode;

typedef uint32_t NativeTimestamp;

typedef enum NativeWindowFrameAction_Tag {
  NativeWindowFrameAction_None,
  /**
   * The window should be minimized.
   */
  NativeWindowFrameAction_Minimize,
  /**
   * The window should be maximized.
   */
  NativeWindowFrameAction_Maximize,
  /**
   * The window should be unmaximized.
   */
  NativeWindowFrameAction_UnMaximize,
  /**
   * The window should be closed.
   */
  NativeWindowFrameAction_Close,
  /**
   * An interactive move should be started.
   */
  NativeWindowFrameAction_Move,
  /**
   * An interactive resize should be started with the provided edge.
   */
  NativeWindowFrameAction_Resize,
  /**
   * Show window menu.
   *
   * The coordinates are relative to the base surface, as in should be
   * directly passed to the `xdg_toplevel::show_window_menu`.
   */
  NativeWindowFrameAction_ShowMenu,
} NativeWindowFrameAction_Tag;

typedef struct NativeWindowFrameAction_NativeShowMenu_Body {
  int32_t _0;
  int32_t _1;
} NativeWindowFrameAction_NativeShowMenu_Body;

typedef struct NativeWindowFrameAction {
  NativeWindowFrameAction_Tag tag;
  union {
    struct {
      enum NativeWindowResizeEdge resize;
    };
    NativeWindowFrameAction_NativeShowMenu_Body show_menu;
  };
} NativeWindowFrameAction;

typedef struct NativeKeyDownEvent {
  struct NativeKeyModifiers modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  bool is_repeat;
  NativeTimestamp timestamp;
  struct NativeWindowFrameAction frame_action_out;
} NativeKeyDownEvent;

typedef struct NativeKeyUpEvent {
  struct NativeKeyModifiers modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  NativeTimestamp timestamp;
} NativeKeyUpEvent;

typedef struct NativeModifiersChangedEvent {
  struct NativeKeyModifiers modifiers;
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
  struct NativeWindowFrameAction frame_action_out;
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

typedef struct NativeWindowCapabilities {
  /**
   * `show_window_menu` is available.
   */
  bool window_menu;
  /**
   * Window can be maximized and unmaximized.
   */
  bool maximixe;
  /**
   * Window can be fullscreened and unfullscreened.
   */
  bool fullscreen;
  /**
   * Window can be minimized.
   */
  bool minimize;
} NativeWindowCapabilities;

typedef struct NativeWindowResizeEvent {
  struct NativeLogicalSize size;
  bool maximized;
  bool fullscreen;
  bool client_side_decorations;
  struct NativeWindowCapabilities capabilities;
} NativeWindowResizeEvent;

typedef struct NativeWindowFocusChangeEvent {
  bool is_key;
  bool is_main;
} NativeWindowFocusChangeEvent;

typedef struct NativeWindowFullScreenToggleEvent {
  bool is_full_screen;
} NativeWindowFullScreenToggleEvent;

typedef struct NativeSoftwareDrawData {
  uint8_t *canvas;
  int32_t stride;
} NativeSoftwareDrawData;

typedef int32_t NativePhysicalPixels;

typedef struct NativePhysicalSize {
  NativePhysicalPixels width;
  NativePhysicalPixels height;
} NativePhysicalSize;

typedef struct NativeWindowDrawEvent {
  struct NativeSoftwareDrawData software_draw_data;
  struct NativePhysicalSize physical_size;
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
      const struct NativeKeyDownEvent *key_down;
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
      const struct NativeMouseDownEvent *mouse_down;
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
  struct NativeLogicalSize size;
  NativeBorrowedStrPtr title;
  /**
   * See <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_app_id>
   */
  NativeBorrowedStrPtr app_id;
  bool force_client_side_decoration;
  bool force_software_rendering;
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

struct NativeGetEglProcFuncData application_get_egl_proc_func(NativeAppPtr app_ptr);

bool application_is_event_loop_thread(NativeAppPtr app_ptr);

void application_run_on_event_loop_async(NativeAppPtr app_ptr, void (*f)(void));

NativeScreenInfoArray screen_list(NativeAppPtr app_ptr);

void screen_list_drop(NativeScreenInfoArray arr);

NativeWindowId window_create(NativeAppPtr app_ptr,
                             NativeEventHandler event_handler,
                             struct NativeWindowParams params);

void window_drop(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_set_pointer_shape(NativeAppPtr app_ptr,
                              NativeWindowId window_id,
                              enum NativePointerShape pointer_shape);

struct NativeLogicalSize window_get_size(NativeAppPtr app_ptr, NativeWindowId window_id);

bool window_is_key(NativeAppPtr app_ptr, NativeWindowId _window_id);

bool window_is_main(NativeAppPtr app_ptr, NativeWindowId _window_id);

void window_set_title(NativeAppPtr app_ptr,
                      NativeWindowId window_id,
                      NativeBorrowedStrPtr new_title);

void window_set_max_size(NativeAppPtr app_ptr,
                         NativeWindowId window_id,
                         struct NativeLogicalSize size);

void window_set_min_size(NativeAppPtr app_ptr,
                         NativeWindowId window_id,
                         struct NativeLogicalSize size);

void window_set_fullscreen(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_unset_fullscreen(NativeAppPtr app_ptr, NativeWindowId window_id);

struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);
