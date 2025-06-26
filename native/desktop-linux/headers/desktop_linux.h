/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef enum NativeDataSource {
  NativeDataSource_Clipboard,
  NativeDataSource_DragAndDrop,
} NativeDataSource;

typedef enum NativeDragAction {
  NativeDragAction_Copy,
  NativeDragAction_Move,
  NativeDragAction_Ask,
} NativeDragAction;

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
   * different from [`PointerShape::Wait`] in that the user may still interact
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

typedef enum NativeTextInputContentPurpose {
  /**
   * default input, allowing all characters
   */
  NativeTextInputContentPurpose_Normal,
  /**
   * allow only alphabetic characters
   */
  NativeTextInputContentPurpose_Alpha,
  /**
   * allow only digits
   */
  NativeTextInputContentPurpose_Digits,
  /**
   * input a number (including decimal separator and sign)
   */
  NativeTextInputContentPurpose_Number,
  /**
   * input a phone number
   */
  NativeTextInputContentPurpose_Phone,
  NativeTextInputContentPurpose_Url,
  /**
   * input an URL
   */
  NativeTextInputContentPurpose_Email,
  /**
   * input an email address
   */
  NativeTextInputContentPurpose_Name,
  /**
   * input a name of a person
   */
  NativeTextInputContentPurpose_Password,
  /**
   * input a password (combine with `sensitive_data` hint)
   */
  NativeTextInputContentPurpose_Pin,
  /**
   * input is a numeric password (combine with `sensitive_data` hint)
   */
  NativeTextInputContentPurpose_Date,
  /**
   * input a date
   */
  NativeTextInputContentPurpose_Time,
  NativeTextInputContentPurpose_Datetime,
  NativeTextInputContentPurpose_Terminal,
} NativeTextInputContentPurpose;

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

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

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
  NativeXdgDesktopSetting_CursorSize,
  NativeXdgDesktopSetting_CursorTheme,
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
      NativeBorrowedStrPtr titlebar_layout;
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
      int32_t cursor_size;
    };
    struct {
      NativeBorrowedStrPtr cursor_theme;
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

typedef uintptr_t NativeArraySize;

typedef struct NativeBorrowedArray_u8 {
  const uint8_t *ptr;
  NativeArraySize len;
  void (*deinit)(const uint8_t*, NativeArraySize);
} NativeBorrowedArray_u8;

typedef struct NativeDataTransferContent {
  int32_t serial;
  struct NativeBorrowedArray_u8 data;
  NativeBorrowedStrPtr mime_types;
} NativeDataTransferContent;

typedef struct NativeDataTransferAvailable {
  NativeBorrowedStrPtr mime_types;
} NativeDataTransferAvailable;

typedef uint32_t NativeKeyCode;

typedef struct NativeKeyDownEvent {
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  uint32_t key;
  bool is_repeat;
} NativeKeyDownEvent;

typedef struct NativeKeyUpEvent {
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  uint32_t key;
} NativeKeyUpEvent;

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

typedef struct NativeModifiersChangedEvent {
  struct NativeKeyModifiers modifiers;
} NativeModifiersChangedEvent;

typedef double NativeLogicalPixels;

typedef struct NativeLogicalPoint {
  NativeLogicalPixels x;
  NativeLogicalPixels y;
} NativeLogicalPoint;

typedef struct NativeMouseEnteredEvent {
  struct NativeLogicalPoint location_in_window;
} NativeMouseEnteredEvent;

typedef struct NativeMouseExitedEvent {
  struct NativeLogicalPoint location_in_window;
} NativeMouseExitedEvent;

typedef uint32_t NativeTimestamp;

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

typedef struct NativeTextInputAvailabilityEvent {
  /**
   * Indicates if the Text Input support is available.
   * Call `application_text_input_enable` to enable it or `application_text_input_disable` to disable it afterward.
   */
  bool available;
} NativeTextInputAvailabilityEvent;

typedef struct NativeTextInputPreeditStringData {
  /**
   * Can be null
   */
  NativeBorrowedStrPtr text;
  int32_t cursor_begin_byte_pos;
  int32_t cursor_end_byte_pos;
} NativeTextInputPreeditStringData;

typedef struct NativeTextInputDeleteSurroundingTextData {
  uint32_t before_length_in_bytes;
  uint32_t after_length_in_bytes;
} NativeTextInputDeleteSurroundingTextData;

/**
 * The application must proceed by evaluating the changes in the following order:
 * 1. Replace the existing preedit string with the cursor.
 * 2. Delete the requested surrounding text.
 * 3. Insert the commit string with the cursor at its end.
 * 4. Calculate surrounding text to send.
 * 5. Insert the new preedit text in the cursor position.
 * 6. Place the cursor inside the preedit text.
 */
typedef struct NativeTextInputEvent {
  bool has_preedit_string;
  struct NativeTextInputPreeditStringData preedit_string;
  bool has_commit_string;
  /**
   * Can be null
   */
  NativeBorrowedStrPtr commit_string;
  bool has_delete_surrounding_text;
  struct NativeTextInputDeleteSurroundingTextData delete_surrounding_text;
} NativeTextInputEvent;

typedef struct NativeLogicalSize {
  NativeLogicalPixels width;
  NativeLogicalPixels height;
} NativeLogicalSize;

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

typedef struct NativeWindowConfigureEvent {
  struct NativeLogicalSize size;
  bool active;
  bool maximized;
  bool fullscreen;
  bool client_side_decorations;
  struct NativeWindowCapabilities capabilities;
} NativeWindowConfigureEvent;

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

typedef struct NativeWindowFocusChangeEvent {
  bool is_key;
  bool is_main;
} NativeWindowFocusChangeEvent;

typedef struct NativeWindowScaleChangedEvent {
  double new_scale;
} NativeWindowScaleChangedEvent;

typedef uint32_t NativeScreenId;

typedef struct NativeWindowScreenChangeEvent {
  NativeScreenId new_screen_id;
} NativeWindowScreenChangeEvent;

typedef enum NativeEvent_Tag {
  NativeEvent_DataTransfer,
  NativeEvent_DataTransferAvailable,
  NativeEvent_KeyDown,
  NativeEvent_KeyUp,
  NativeEvent_ModifiersChanged,
  NativeEvent_MouseEntered,
  NativeEvent_MouseExited,
  NativeEvent_MouseMoved,
  NativeEvent_MouseDragged,
  NativeEvent_MouseDown,
  NativeEvent_MouseUp,
  NativeEvent_ScrollWheel,
  NativeEvent_TextInputAvailability,
  NativeEvent_TextInput,
  NativeEvent_WindowCloseRequest,
  NativeEvent_WindowConfigure,
  NativeEvent_WindowDraw,
  NativeEvent_WindowFocusChange,
  NativeEvent_WindowScaleChanged,
  NativeEvent_WindowScreenChange,
} NativeEvent_Tag;

typedef struct NativeEvent {
  NativeEvent_Tag tag;
  union {
    struct {
      struct NativeDataTransferContent data_transfer;
    };
    struct {
      struct NativeDataTransferAvailable data_transfer_available;
    };
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
      struct NativeMouseEnteredEvent mouse_entered;
    };
    struct {
      struct NativeMouseExitedEvent mouse_exited;
    };
    struct {
      struct NativeMouseMovedEvent mouse_moved;
    };
    struct {
      struct NativeMouseDraggedEvent mouse_dragged;
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
      struct NativeTextInputAvailabilityEvent text_input_availability;
    };
    struct {
      struct NativeTextInputEvent text_input;
    };
    struct {
      struct NativeWindowConfigureEvent window_configure;
    };
    struct {
      struct NativeWindowDrawEvent window_draw;
    };
    struct {
      struct NativeWindowFocusChangeEvent window_focus_change;
    };
    struct {
      struct NativeWindowScaleChangedEvent window_scale_changed;
    };
    struct {
      struct NativeWindowScreenChangeEvent window_screen_change;
    };
  };
} NativeEvent;

typedef int64_t NativeWindowId;

typedef bool (*NativeEventHandler)(const struct NativeEvent*, NativeWindowId);

typedef struct NativeDragAndDropQueryData {
  NativeWindowId window_id;
  struct NativeLogicalPoint point;
} NativeDragAndDropQueryData;

typedef struct NativeApplicationCallbacks {
  void (*on_application_started)(void);
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  void (*on_display_configuration_change)(void);
  void (*on_xdg_desktop_settings_change)(const struct NativeXdgDesktopSetting*);
  NativeEventHandler event_handler;
  NativeBorrowedStrPtr (*get_drag_and_drop_supported_mime_types)(const struct NativeDragAndDropQueryData*);
  struct NativeBorrowedArray_u8 (*get_data_transfer_data)(enum NativeDataSource,
                                                          NativeBorrowedStrPtr);
  void (*on_data_transfer_cancelled)(enum NativeDataSource);
} NativeApplicationCallbacks;

typedef NativeGenericRawPtr_c_void NativeBorrowedOpaquePtr;

typedef struct NativeGetEglProcFuncData {
  void (*(*f)(NativeBorrowedOpaquePtr ctx, NativeBorrowedStrPtr name))(void);
  NativeBorrowedOpaquePtr ctx;
} NativeGetEglProcFuncData;

typedef struct NativeLogicalRect {
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
} NativeLogicalRect;

typedef struct NativeTextInputContext {
  NativeBorrowedStrPtr surrounding_text;
  uint16_t cursor_codepoint_offset;
  uint16_t selection_start_codepoint_offset;
  bool is_multiline;
  enum NativeTextInputContentPurpose content_purpose;
  struct NativeLogicalRect cursor_rectangle;
  bool change_caused_by_input_method;
} NativeTextInputContext;

typedef NativeGenericRawPtr_c_char NativeRustAllocatedStrPtr;

typedef NativeRustAllocatedStrPtr NativeAutoDropStrPtr;

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

typedef struct NativeWindowParams {
  NativeWindowId window_id;
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

void application_set_cursor_theme(NativeAppPtr app_ptr, NativeBorrowedStrPtr name, uint32_t size);

void application_text_input_enable(NativeAppPtr app_ptr, struct NativeTextInputContext context);

void application_text_input_update(NativeAppPtr app_ptr, struct NativeTextInputContext context);

void application_text_input_disable(NativeAppPtr app_ptr);

void application_clipboard_put(NativeAppPtr app_ptr, NativeBorrowedStrPtr mime_types);

void application_start_drag_and_drop(NativeAppPtr app_ptr,
                                     NativeWindowId window_id,
                                     NativeBorrowedStrPtr mime_types,
                                     enum NativeDragAction action);

bool application_open_url(NativeBorrowedStrPtr url_string);

NativeScreenInfoArray screen_list(NativeAppPtr app_ptr);

void screen_list_drop(NativeScreenInfoArray arr);

void window_create(NativeAppPtr app_ptr, struct NativeWindowParams params);

void window_close(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_set_pointer_shape(NativeAppPtr app_ptr,
                              NativeWindowId window_id,
                              enum NativePointerShape pointer_shape);

struct NativeLogicalSize window_get_size(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_set_title(NativeAppPtr app_ptr,
                      NativeWindowId window_id,
                      NativeBorrowedStrPtr new_title);

void window_start_move(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_start_resize(NativeAppPtr app_ptr,
                         NativeWindowId window_id,
                         enum NativeWindowResizeEdge edge);

void window_show_menu(NativeAppPtr app_ptr,
                      NativeWindowId window_id,
                      struct NativeLogicalPoint position);

void window_maximize(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_unmaximize(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_minimize(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_set_max_size(NativeAppPtr app_ptr,
                         NativeWindowId window_id,
                         struct NativeLogicalSize size);

void window_set_min_size(NativeAppPtr app_ptr,
                         NativeWindowId window_id,
                         struct NativeLogicalSize size);

void window_set_fullscreen(NativeAppPtr app_ptr, NativeWindowId window_id);

void window_unset_fullscreen(NativeAppPtr app_ptr, NativeWindowId window_id);

bool window_clipboard_paste(NativeAppPtr app_ptr,
                            NativeWindowId window_id,
                            int32_t serial,
                            NativeBorrowedStrPtr supported_mime_types);

struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);
