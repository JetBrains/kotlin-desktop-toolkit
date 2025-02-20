/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

#define NativeCapsLockModifier (1 << 16)

#define NativeShiftModifier (1 << 17)

#define NativeControlModifier (1 << 18)

#define NativeOptionModifier (1 << 19)

#define NativeCommandModifier (1 << 20)

#define NativeNumericPadModifier (1 << 21)

#define NativeHelpModifier (1 << 22)

#define NativeFunctionModifier (1 << 23)

#define NativeEnterCharacter 3

#define NativeBackspaceCharacter 8

#define NativeTabCharacter 9

#define NativeNewlineCharacter 10

#define NativeFormFeedCharacter 12

#define NativeCarriageReturnCharacter 13

#define NativeBackTabCharacter 25

#define NativeDeleteCharacter 127

#define NativeLineSeparatorCharacter 8232

#define NativeParagraphSeparatorCharacter 8233

#define NativeUpArrowFunctionKey 63232

#define NativeDownArrowFunctionKey 63233

#define NativeLeftArrowFunctionKey 63234

#define NativeRightArrowFunctionKey 63235

#define NativeF1FunctionKey 63236

#define NativeF2FunctionKey 63237

#define NativeF3FunctionKey 63238

#define NativeF4FunctionKey 63239

#define NativeF5FunctionKey 63240

#define NativeF6FunctionKey 63241

#define NativeF7FunctionKey 63242

#define NativeF8FunctionKey 63243

#define NativeF9FunctionKey 63244

#define NativeF10FunctionKey 63245

#define NativeF11FunctionKey 63246

#define NativeF12FunctionKey 63247

#define NativeF13FunctionKey 63248

#define NativeF14FunctionKey 63249

#define NativeF15FunctionKey 63250

#define NativeF16FunctionKey 63251

#define NativeF17FunctionKey 63252

#define NativeF18FunctionKey 63253

#define NativeF19FunctionKey 63254

#define NativeF20FunctionKey 63255

#define NativeF21FunctionKey 63256

#define NativeF22FunctionKey 63257

#define NativeF23FunctionKey 63258

#define NativeF24FunctionKey 63259

#define NativeF25FunctionKey 63260

#define NativeF26FunctionKey 63261

#define NativeF27FunctionKey 63262

#define NativeF28FunctionKey 63263

#define NativeF29FunctionKey 63264

#define NativeF30FunctionKey 63265

#define NativeF31FunctionKey 63266

#define NativeF32FunctionKey 63267

#define NativeF33FunctionKey 63268

#define NativeF34FunctionKey 63269

#define NativeF35FunctionKey 63270

#define NativeInsertFunctionKey 63271

#define NativeDeleteFunctionKey 63272

#define NativeHomeFunctionKey 63273

#define NativeBeginFunctionKey 63274

#define NativeEndFunctionKey 63275

#define NativePageUpFunctionKey 63276

#define NativePageDownFunctionKey 63277

#define NativePrintScreenFunctionKey 63278

#define NativeScrollLockFunctionKey 63279

#define NativePauseFunctionKey 63280

#define NativeSysReqFunctionKey 63281

#define NativeBreakFunctionKey 63282

#define NativeResetFunctionKey 63283

#define NativeStopFunctionKey 63284

#define NativeMenuFunctionKey 63285

#define NativeUserFunctionKey 63286

#define NativeSystemFunctionKey 63287

#define NativePrintFunctionKey 63288

#define NativeClearLineFunctionKey 63289

#define NativeClearDisplayFunctionKey 63290

#define NativeInsertLineFunctionKey 63291

#define NativeDeleteLineFunctionKey 63292

#define NativeInsertCharFunctionKey 63293

#define NativeDeleteCharFunctionKey 63294

#define NativePrevFunctionKey 63295

#define NativeNextFunctionKey 63296

#define NativeSelectFunctionKey 63297

#define NativeExecuteFunctionKey 63298

#define NativeUndoFunctionKey 63299

#define NativeRedoFunctionKey 63300

#define NativeFindFunctionKey 63301

#define NativeHelpFunctionKey 63302

#define NativeModeSwitchFunctionKey 63303

typedef enum NativeKeyCode {
  NativeKeyCode_VK_ANSI_A = 0,
  NativeKeyCode_VK_ANSI_S = 1,
  NativeKeyCode_VK_ANSI_D = 2,
  NativeKeyCode_VK_ANSI_F = 3,
  NativeKeyCode_VK_ANSI_H = 4,
  NativeKeyCode_VK_ANSI_G = 5,
  NativeKeyCode_VK_ANSI_Z = 6,
  NativeKeyCode_VK_ANSI_X = 7,
  NativeKeyCode_VK_ANSI_C = 8,
  NativeKeyCode_VK_ANSI_V = 9,
  NativeKeyCode_VK_ANSI_B = 11,
  NativeKeyCode_VK_ANSI_Q = 12,
  NativeKeyCode_VK_ANSI_W = 13,
  NativeKeyCode_VK_ANSI_E = 14,
  NativeKeyCode_VK_ANSI_R = 15,
  NativeKeyCode_VK_ANSI_Y = 16,
  NativeKeyCode_VK_ANSI_T = 17,
  NativeKeyCode_VK_ANSI_1 = 18,
  NativeKeyCode_VK_ANSI_2 = 19,
  NativeKeyCode_VK_ANSI_3 = 20,
  NativeKeyCode_VK_ANSI_4 = 21,
  NativeKeyCode_VK_ANSI_6 = 22,
  NativeKeyCode_VK_ANSI_5 = 23,
  NativeKeyCode_VK_ANSI_Equal = 24,
  NativeKeyCode_VK_ANSI_9 = 25,
  NativeKeyCode_VK_ANSI_7 = 26,
  NativeKeyCode_VK_ANSI_Minus = 27,
  NativeKeyCode_VK_ANSI_8 = 28,
  NativeKeyCode_VK_ANSI_0 = 29,
  NativeKeyCode_VK_ANSI_RightBracket = 30,
  NativeKeyCode_VK_ANSI_O = 31,
  NativeKeyCode_VK_ANSI_U = 32,
  NativeKeyCode_VK_ANSI_LeftBracket = 33,
  NativeKeyCode_VK_ANSI_I = 34,
  NativeKeyCode_VK_ANSI_P = 35,
  NativeKeyCode_VK_ANSI_L = 37,
  NativeKeyCode_VK_ANSI_J = 38,
  NativeKeyCode_VK_ANSI_Quote = 39,
  NativeKeyCode_VK_ANSI_K = 40,
  NativeKeyCode_VK_ANSI_Semicolon = 41,
  NativeKeyCode_VK_ANSI_Backslash = 42,
  NativeKeyCode_VK_ANSI_Comma = 43,
  NativeKeyCode_VK_ANSI_Slash = 44,
  NativeKeyCode_VK_ANSI_N = 45,
  NativeKeyCode_VK_ANSI_M = 46,
  NativeKeyCode_VK_ANSI_Period = 47,
  NativeKeyCode_VK_ANSI_Grave = 50,
  NativeKeyCode_VK_ANSI_KeypadDecimal = 65,
  NativeKeyCode_VK_ANSI_KeypadMultiply = 67,
  NativeKeyCode_VK_ANSI_KeypadPlus = 69,
  NativeKeyCode_VK_ANSI_KeypadClear = 71,
  NativeKeyCode_VK_ANSI_KeypadDivide = 75,
  NativeKeyCode_VK_ANSI_KeypadEnter = 76,
  NativeKeyCode_VK_ANSI_KeypadMinus = 78,
  NativeKeyCode_VK_ANSI_KeypadEquals = 81,
  NativeKeyCode_VK_ANSI_Keypad0 = 82,
  NativeKeyCode_VK_ANSI_Keypad1 = 83,
  NativeKeyCode_VK_ANSI_Keypad2 = 84,
  NativeKeyCode_VK_ANSI_Keypad3 = 85,
  NativeKeyCode_VK_ANSI_Keypad4 = 86,
  NativeKeyCode_VK_ANSI_Keypad5 = 87,
  NativeKeyCode_VK_ANSI_Keypad6 = 88,
  NativeKeyCode_VK_ANSI_Keypad7 = 89,
  NativeKeyCode_VK_ANSI_Keypad8 = 91,
  NativeKeyCode_VK_ANSI_Keypad9 = 92,
  NativeKeyCode_VK_Return = 36,
  NativeKeyCode_VK_Tab = 48,
  NativeKeyCode_VK_Space = 49,
  NativeKeyCode_VK_Delete = 51,
  NativeKeyCode_VK_Escape = 53,
  NativeKeyCode_VK_Command = 55,
  NativeKeyCode_VK_Shift = 56,
  NativeKeyCode_VK_CapsLock = 57,
  NativeKeyCode_VK_Option = 58,
  NativeKeyCode_VK_Control = 59,
  NativeKeyCode_VK_RightCommand = 54,
  NativeKeyCode_VK_RightShift = 60,
  NativeKeyCode_VK_RightOption = 61,
  NativeKeyCode_VK_RightControl = 62,
  NativeKeyCode_VK_Function = 63,
  NativeKeyCode_VK_F17 = 64,
  NativeKeyCode_VK_VolumeUp = 72,
  NativeKeyCode_VK_VolumeDown = 73,
  NativeKeyCode_VK_Mute = 74,
  NativeKeyCode_VK_F18 = 79,
  NativeKeyCode_VK_F19 = 80,
  NativeKeyCode_VK_F20 = 90,
  NativeKeyCode_VK_F5 = 96,
  NativeKeyCode_VK_F6 = 97,
  NativeKeyCode_VK_F7 = 98,
  NativeKeyCode_VK_F3 = 99,
  NativeKeyCode_VK_F8 = 100,
  NativeKeyCode_VK_F9 = 101,
  NativeKeyCode_VK_F11 = 103,
  NativeKeyCode_VK_F13 = 105,
  NativeKeyCode_VK_F16 = 106,
  NativeKeyCode_VK_F14 = 107,
  NativeKeyCode_VK_F10 = 109,
  NativeKeyCode_VK_ContextualMenu = 110,
  NativeKeyCode_VK_F12 = 111,
  NativeKeyCode_VK_F15 = 113,
  NativeKeyCode_VK_Help = 114,
  NativeKeyCode_VK_Home = 115,
  NativeKeyCode_VK_PageUp = 116,
  NativeKeyCode_VK_ForwardDelete = 117,
  NativeKeyCode_VK_F4 = 118,
  NativeKeyCode_VK_End = 119,
  NativeKeyCode_VK_F2 = 120,
  NativeKeyCode_VK_PageDown = 121,
  NativeKeyCode_VK_F1 = 122,
  NativeKeyCode_VK_LeftArrow = 123,
  NativeKeyCode_VK_RightArrow = 124,
  NativeKeyCode_VK_DownArrow = 125,
  NativeKeyCode_VK_UpArrow = 126,
  NativeKeyCode_VK_ISO_Section = 10,
  NativeKeyCode_VK_JIS_Yen = 93,
  NativeKeyCode_VK_JIS_Underscore = 94,
  NativeKeyCode_VK_JIS_KeypadComma = 95,
  NativeKeyCode_VK_JIS_Eisu = 102,
  NativeKeyCode_VK_JIS_Kana = 104,
} NativeKeyCode;

typedef enum NativeLogLevel {
  NativeLogLevel_Off,
  NativeLogLevel_Error,
  NativeLogLevel_Warn,
  NativeLogLevel_Info,
  NativeLogLevel_Debug,
  NativeLogLevel_Trace,
} NativeLogLevel;

typedef enum NativeWindowVisualEffect {
  NativeWindowVisualEffect_TitlebarEffect,
  NativeWindowVisualEffect_SelectionEffect,
  NativeWindowVisualEffect_MenuEffect,
  NativeWindowVisualEffect_PopoverEffect,
  NativeWindowVisualEffect_SidebarEffect,
  NativeWindowVisualEffect_HeaderViewEffect,
  NativeWindowVisualEffect_SheetEffect,
  NativeWindowVisualEffect_WindowBackgroundEffect,
  NativeWindowVisualEffect_HUDWindowEffect,
  NativeWindowVisualEffect_FullScreenUIEffect,
  NativeWindowVisualEffect_ToolTipEffect,
  NativeWindowVisualEffect_ContentBackgroundEffect,
  NativeWindowVisualEffect_UnderWindowBackgroundEffect,
  NativeWindowVisualEffect_UnderPageBackgroundEffect,
} NativeWindowVisualEffect;

typedef struct NativeDisplayLinkBox NativeDisplayLinkBox;

typedef struct NativeMetalView NativeMetalView;

typedef struct NativeWindow NativeWindow;

typedef char *NativeStrPtr;

typedef int64_t NativeArraySize;

typedef struct NativeExceptionsArray {
  const NativeStrPtr *items;
  NativeArraySize count;
} NativeExceptionsArray;

typedef struct NativeLoggerConfiguration {
  NativeStrPtr file_path;
  enum NativeLogLevel console_level;
  enum NativeLogLevel file_level;
} NativeLoggerConfiguration;

typedef struct NativeApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} NativeApplicationConfig;

typedef int64_t NativeWindowId;

typedef uint32_t NativeKeyModifiersSet;

typedef double NativeTimestamp;

typedef struct NativeKeyDownEvent {
  NativeWindowId window_id;
  NativeKeyModifiersSet modifiers;
  enum NativeKeyCode code;
  NativeStrPtr characters;
  NativeStrPtr key;
  bool is_repeat;
  NativeTimestamp timestamp;
} NativeKeyDownEvent;

typedef struct NativeKeyUpEvent {
  NativeWindowId window_id;
  NativeKeyModifiersSet modifiers;
  enum NativeKeyCode code;
  NativeStrPtr characters;
  NativeStrPtr key;
  NativeTimestamp timestamp;
} NativeKeyUpEvent;

typedef struct NativeModifiersChangedEvent {
  NativeWindowId window_id;
  NativeKeyModifiersSet modifiers;
  enum NativeKeyCode code;
  NativeTimestamp timestamp;
} NativeModifiersChangedEvent;

typedef double NativeLogicalPixels;

typedef struct NativeLogicalPoint {
  NativeLogicalPixels x;
  NativeLogicalPixels y;
} NativeLogicalPoint;

typedef struct NativeMouseMovedEvent {
  NativeWindowId window_id;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseMovedEvent;

typedef uint32_t NativeMouseButton;

typedef struct NativeMouseDraggedEvent {
  NativeWindowId window_id;
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseDraggedEvent;

typedef struct NativeMouseEnteredEvent {
  NativeWindowId window_id;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseEnteredEvent;

typedef struct NativeMouseExitedEvent {
  NativeWindowId window_id;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseExitedEvent;

typedef struct NativeMouseDownEvent {
  NativeWindowId window_id;
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseDownEvent;

typedef struct NativeMouseUpEvent {
  NativeWindowId window_id;
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeMouseUpEvent;

typedef struct NativeScrollWheelEvent {
  NativeWindowId window_id;
  NativeLogicalPixels scrolling_delta_x;
  NativeLogicalPixels scrolling_delta_y;
  bool has_precise_scrolling_deltas;
  struct NativeLogicalPoint location_in_window;
  NativeTimestamp timestamp;
} NativeScrollWheelEvent;

typedef uint32_t NativeScreenId;

typedef struct NativeWindowScreenChangeEvent {
  NativeWindowId window_id;
  NativeScreenId new_screen_id;
} NativeWindowScreenChangeEvent;

typedef struct NativeLogicalSize {
  NativeLogicalPixels width;
  NativeLogicalPixels height;
} NativeLogicalSize;

typedef struct NativeWindowResizeEvent {
  NativeWindowId window_id;
  struct NativeLogicalSize size;
} NativeWindowResizeEvent;

typedef struct NativeWindowMoveEvent {
  NativeWindowId window_id;
  struct NativeLogicalPoint origin;
} NativeWindowMoveEvent;

typedef struct NativeWindowFocusChangeEvent {
  NativeWindowId window_id;
  bool is_key;
  bool is_main;
} NativeWindowFocusChangeEvent;

typedef struct NativeWindowCloseRequestEvent {
  NativeWindowId window_id;
} NativeWindowCloseRequestEvent;

typedef struct NativeWindowFullScreenToggleEvent {
  NativeWindowId window_id;
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
  NativeEvent_DisplayConfigurationChange,
  NativeEvent_ApplicationDidFinishLaunching,
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

typedef const char *NativeConstStrPtr;

typedef struct NativeTextCommandOperation {
  NativeWindowId window_id;
  NativeConstStrPtr command;
} NativeTextCommandOperation;

typedef struct NativeTextChangedOperation {
  NativeWindowId window_id;
  NativeConstStrPtr text;
} NativeTextChangedOperation;

typedef enum NativeTextOperation_Tag {
  NativeTextOperation_TextCommand,
  NativeTextOperation_TextChanged,
} NativeTextOperation_Tag;

typedef struct NativeTextOperation {
  NativeTextOperation_Tag tag;
  union {
    struct {
      struct NativeTextCommandOperation text_command;
    };
    struct {
      struct NativeTextChangedOperation text_changed;
    };
  };
} NativeTextOperation;

typedef bool (*NativeTextOperationHandler)(const struct NativeTextOperation*);

typedef struct NativeApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  NativeEventHandler event_handler;
  NativeTextOperationHandler text_operation_handler;
} NativeApplicationCallbacks;

typedef struct NativeAppMenuKeystroke {
  NativeStrPtr key;
  NativeKeyModifiersSet modifiers;
} NativeAppMenuKeystroke;

typedef enum NativeAppMenuItem_Tag {
  NativeAppMenuItem_ActionItem,
  NativeAppMenuItem_SeparatorItem,
  NativeAppMenuItem_SubMenuItem,
} NativeAppMenuItem_Tag;

typedef struct NativeAppMenuItem_NativeActionItem_Body {
  bool enabled;
  NativeStrPtr title;
  bool macos_provided;
  const struct NativeAppMenuKeystroke *keystroke;
  void (*perform)(void);
} NativeAppMenuItem_NativeActionItem_Body;

typedef struct NativeAppMenuItem_NativeSubMenuItem_Body {
  NativeStrPtr title;
  NativeStrPtr special_tag;
  const struct NativeAppMenuItem *items;
  NativeArraySize items_count;
} NativeAppMenuItem_NativeSubMenuItem_Body;

typedef struct NativeAppMenuItem {
  NativeAppMenuItem_Tag tag;
  union {
    NativeAppMenuItem_NativeActionItem_Body action_item;
    NativeAppMenuItem_NativeSubMenuItem_Body sub_menu_item;
  };
} NativeAppMenuItem;

typedef struct NativeAppMenuStructure {
  const struct NativeAppMenuItem *items;
  NativeArraySize items_count;
} NativeAppMenuStructure;

typedef void (*NativeDisplayLinkCallback)(void);

typedef uint32_t NativeMouseButtonsSet;

typedef void *NativeMetalDeviceRef;

typedef void *NativeMetalCommandQueueRef;

typedef double NativePhysicalPixels;

typedef struct NativePhysicalSize {
  NativePhysicalPixels width;
  NativePhysicalPixels height;
} NativePhysicalSize;

typedef void *NativeMetalTextureRef;

typedef struct NativeScreenInfo {
  NativeScreenId screen_id;
  bool is_primary;
  NativeStrPtr name;
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
  double scale;
  uint32_t maximum_frames_per_second;
} NativeScreenInfo;

typedef struct NativeScreenInfoArray {
  struct NativeScreenInfo *ptr;
  NativeArraySize len;
} NativeScreenInfoArray;

typedef struct NativeWindowParams {
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
  NativeStrPtr title;
  bool is_resizable;
  bool is_closable;
  bool is_miniaturizable;
  bool is_full_screen_allowed;
  bool use_custom_titlebar;
  NativeLogicalPixels titlebar_height;
} NativeWindowParams;

typedef struct NativeColor {
  double red;
  double green;
  double blue;
  double alpha;
} NativeColor;

typedef enum NativeWindowBackground_Tag {
  NativeWindowBackground_Transparent,
  NativeWindowBackground_SolidColor,
  NativeWindowBackground_VisualEffect,
} NativeWindowBackground_Tag;

typedef struct NativeWindowBackground {
  NativeWindowBackground_Tag tag;
  union {
    struct {
      struct NativeColor solid_color;
    };
    struct {
      enum NativeWindowVisualEffect visual_effect;
    };
  };
} NativeWindowBackground;

#define NativeLeftMouseButton 0

#define NativeRightMouseButton 1

#define NativeMiddleMouseButton 2

struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);

void application_init(const struct NativeApplicationConfig *config,
                      struct NativeApplicationCallbacks callbacks);

void application_shutdown(void);

void application_run_event_loop(void);

void application_stop_event_loop(void);

void application_request_termination(void);

void main_menu_update(struct NativeAppMenuStructure menu);

void main_menu_set_none(void);

bool dispatcher_is_main_thread(void);

void dispatcher_main_exec_async(void (*f)(void));

struct NativeDisplayLinkBox *display_link_create(NativeScreenId screen_id,
                                                 NativeDisplayLinkCallback on_next_frame);

void display_link_drop(struct NativeDisplayLinkBox *display_link);

void display_link_set_running(struct NativeDisplayLinkBox *display_link, bool value);

bool display_link_is_running(struct NativeDisplayLinkBox *display_link);

NativeMouseButtonsSet events_pressed_mouse_buttons(void);

NativeKeyModifiersSet events_pressed_modifiers(void);

struct NativeLogicalPoint events_cursor_location_in_screen(void);

NativeMetalDeviceRef metal_create_device(void);

void metal_deref_device(NativeMetalDeviceRef device);

NativeMetalCommandQueueRef metal_create_command_queue(NativeMetalDeviceRef device);

void metal_deref_command_queue(NativeMetalCommandQueueRef queue);

struct NativeMetalView *metal_create_view(NativeMetalDeviceRef device);

void metal_drop_view(struct NativeMetalView *view);

void metal_view_set_is_opaque(const struct NativeMetalView *view, bool value);

bool metal_view_get_is_opaque(const struct NativeMetalView *view);

void metal_view_present(const struct NativeMetalView *view,
                        NativeMetalCommandQueueRef queue,
                        bool wait_for_ca_transaction);

struct NativePhysicalSize metal_view_get_texture_size(const struct NativeMetalView *view);

NativeMetalTextureRef metal_view_next_texture(const struct NativeMetalView *view);

void metal_deref_texture(NativeMetalTextureRef texture);

struct NativeScreenInfoArray screen_list(void);

void screen_list_drop(struct NativeScreenInfoArray arr);

NativeScreenId screen_get_main_screen_id(void);

void string_drop(NativeStrPtr str_ptr);

struct NativeWindow *window_create(const struct NativeWindowParams *params);

void window_drop(struct NativeWindow *window);

NativeWindowId window_get_window_id(const struct NativeWindow *window);

NativeScreenId window_get_screen_id(const struct NativeWindow *window);

double window_scale_factor(const struct NativeWindow *window);

void window_attach_layer(const struct NativeWindow *window, const struct NativeMetalView *layer);

void window_set_title(const struct NativeWindow *window, NativeStrPtr new_title);

NativeStrPtr window_get_title(const struct NativeWindow *window);

struct NativeLogicalPoint window_get_origin(const struct NativeWindow *window);

struct NativeLogicalSize window_get_size(const struct NativeWindow *window);

void window_set_rect(const struct NativeWindow *window,
                     struct NativeLogicalPoint origin,
                     struct NativeLogicalSize size,
                     bool animate);

struct NativeLogicalPoint window_get_content_origin(const struct NativeWindow *window);

struct NativeLogicalSize window_get_content_size(const struct NativeWindow *window);

void window_set_content_rect(const struct NativeWindow *window,
                             struct NativeLogicalPoint origin,
                             struct NativeLogicalSize size,
                             bool animate);

bool window_is_key(const struct NativeWindow *window);

bool window_is_main(const struct NativeWindow *window);

struct NativeLogicalSize window_get_max_size(const struct NativeWindow *window);

void window_set_max_size(const struct NativeWindow *window, struct NativeLogicalSize size);

struct NativeLogicalSize window_get_min_size(const struct NativeWindow *window);

void window_set_min_size(const struct NativeWindow *window, struct NativeLogicalSize size);

void window_toggle_full_screen(const struct NativeWindow *window);

bool window_is_full_screen(const struct NativeWindow *window);

void window_start_drag(const struct NativeWindow *window);

void window_invalidate_shadow(const struct NativeWindow *window);

void window_set_background(const struct NativeWindow *window,
                           struct NativeWindowBackground background);
