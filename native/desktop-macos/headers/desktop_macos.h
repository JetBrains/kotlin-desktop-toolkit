/* This header is generated please don't edit it manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define EnterCharacter 3

#define BackspaceCharacter 8

#define TabCharacter 9

#define NewlineCharacter 10

#define FormFeedCharacter 12

#define CarriageReturnCharacter 13

#define BackTabCharacter 25

#define DeleteCharacter 127

#define LineSeparatorCharacter 8232

#define ParagraphSeparatorCharacter 8233

#define UpArrowFunctionKey 63232

#define DownArrowFunctionKey 63233

#define LeftArrowFunctionKey 63234

#define RightArrowFunctionKey 63235

#define F1FunctionKey 63236

#define F2FunctionKey 63237

#define F3FunctionKey 63238

#define F4FunctionKey 63239

#define F5FunctionKey 63240

#define F6FunctionKey 63241

#define F7FunctionKey 63242

#define F8FunctionKey 63243

#define F9FunctionKey 63244

#define F10FunctionKey 63245

#define F11FunctionKey 63246

#define F12FunctionKey 63247

#define F13FunctionKey 63248

#define F14FunctionKey 63249

#define F15FunctionKey 63250

#define F16FunctionKey 63251

#define F17FunctionKey 63252

#define F18FunctionKey 63253

#define F19FunctionKey 63254

#define F20FunctionKey 63255

#define F21FunctionKey 63256

#define F22FunctionKey 63257

#define F23FunctionKey 63258

#define F24FunctionKey 63259

#define F25FunctionKey 63260

#define F26FunctionKey 63261

#define F27FunctionKey 63262

#define F28FunctionKey 63263

#define F29FunctionKey 63264

#define F30FunctionKey 63265

#define F31FunctionKey 63266

#define F32FunctionKey 63267

#define F33FunctionKey 63268

#define F34FunctionKey 63269

#define F35FunctionKey 63270

#define InsertFunctionKey 63271

#define DeleteFunctionKey 63272

#define HomeFunctionKey 63273

#define BeginFunctionKey 63274

#define EndFunctionKey 63275

#define PageUpFunctionKey 63276

#define PageDownFunctionKey 63277

#define PrintScreenFunctionKey 63278

#define ScrollLockFunctionKey 63279

#define PauseFunctionKey 63280

#define SysReqFunctionKey 63281

#define BreakFunctionKey 63282

#define ResetFunctionKey 63283

#define StopFunctionKey 63284

#define MenuFunctionKey 63285

#define UserFunctionKey 63286

#define SystemFunctionKey 63287

#define PrintFunctionKey 63288

#define ClearLineFunctionKey 63289

#define ClearDisplayFunctionKey 63290

#define InsertLineFunctionKey 63291

#define DeleteLineFunctionKey 63292

#define InsertCharFunctionKey 63293

#define DeleteCharFunctionKey 63294

#define PrevFunctionKey 63295

#define NextFunctionKey 63296

#define SelectFunctionKey 63297

#define ExecuteFunctionKey 63298

#define UndoFunctionKey 63299

#define RedoFunctionKey 63300

#define FindFunctionKey 63301

#define HelpFunctionKey 63302

#define ModeSwitchFunctionKey 63303

typedef enum KeyCode {
  VK_ANSI_A = 0,
  VK_ANSI_S = 1,
  VK_ANSI_D = 2,
  VK_ANSI_F = 3,
  VK_ANSI_H = 4,
  VK_ANSI_G = 5,
  VK_ANSI_Z = 6,
  VK_ANSI_X = 7,
  VK_ANSI_C = 8,
  VK_ANSI_V = 9,
  VK_ANSI_B = 11,
  VK_ANSI_Q = 12,
  VK_ANSI_W = 13,
  VK_ANSI_E = 14,
  VK_ANSI_R = 15,
  VK_ANSI_Y = 16,
  VK_ANSI_T = 17,
  VK_ANSI_1 = 18,
  VK_ANSI_2 = 19,
  VK_ANSI_3 = 20,
  VK_ANSI_4 = 21,
  VK_ANSI_6 = 22,
  VK_ANSI_5 = 23,
  VK_ANSI_Equal = 24,
  VK_ANSI_9 = 25,
  VK_ANSI_7 = 26,
  VK_ANSI_Minus = 27,
  VK_ANSI_8 = 28,
  VK_ANSI_0 = 29,
  VK_ANSI_RightBracket = 30,
  VK_ANSI_O = 31,
  VK_ANSI_U = 32,
  VK_ANSI_LeftBracket = 33,
  VK_ANSI_I = 34,
  VK_ANSI_P = 35,
  VK_ANSI_L = 37,
  VK_ANSI_J = 38,
  VK_ANSI_Quote = 39,
  VK_ANSI_K = 40,
  VK_ANSI_Semicolon = 41,
  VK_ANSI_Backslash = 42,
  VK_ANSI_Comma = 43,
  VK_ANSI_Slash = 44,
  VK_ANSI_N = 45,
  VK_ANSI_M = 46,
  VK_ANSI_Period = 47,
  VK_ANSI_Grave = 50,
  VK_ANSI_KeypadDecimal = 65,
  VK_ANSI_KeypadMultiply = 67,
  VK_ANSI_KeypadPlus = 69,
  VK_ANSI_KeypadClear = 71,
  VK_ANSI_KeypadDivide = 75,
  VK_ANSI_KeypadEnter = 76,
  VK_ANSI_KeypadMinus = 78,
  VK_ANSI_KeypadEquals = 81,
  VK_ANSI_Keypad0 = 82,
  VK_ANSI_Keypad1 = 83,
  VK_ANSI_Keypad2 = 84,
  VK_ANSI_Keypad3 = 85,
  VK_ANSI_Keypad4 = 86,
  VK_ANSI_Keypad5 = 87,
  VK_ANSI_Keypad6 = 88,
  VK_ANSI_Keypad7 = 89,
  VK_ANSI_Keypad8 = 91,
  VK_ANSI_Keypad9 = 92,
  VK_Return = 36,
  VK_Tab = 48,
  VK_Space = 49,
  VK_Delete = 51,
  VK_Escape = 53,
  VK_Command = 55,
  VK_Shift = 56,
  VK_CapsLock = 57,
  VK_Option = 58,
  VK_Control = 59,
  VK_RightCommand = 54,
  VK_RightShift = 60,
  VK_RightOption = 61,
  VK_RightControl = 62,
  VK_Function = 63,
  VK_F17 = 64,
  VK_VolumeUp = 72,
  VK_VolumeDown = 73,
  VK_Mute = 74,
  VK_F18 = 79,
  VK_F19 = 80,
  VK_F20 = 90,
  VK_F5 = 96,
  VK_F6 = 97,
  VK_F7 = 98,
  VK_F3 = 99,
  VK_F8 = 100,
  VK_F9 = 101,
  VK_F11 = 103,
  VK_F13 = 105,
  VK_F16 = 106,
  VK_F14 = 107,
  VK_F10 = 109,
  VK_ContextualMenu = 110,
  VK_F12 = 111,
  VK_F15 = 113,
  VK_Help = 114,
  VK_Home = 115,
  VK_PageUp = 116,
  VK_ForwardDelete = 117,
  VK_F4 = 118,
  VK_End = 119,
  VK_F2 = 120,
  VK_PageDown = 121,
  VK_F1 = 122,
  VK_LeftArrow = 123,
  VK_RightArrow = 124,
  VK_DownArrow = 125,
  VK_UpArrow = 126,
  VK_ISO_Section = 10,
  VK_JIS_Yen = 93,
  VK_JIS_Underscore = 94,
  VK_JIS_KeypadComma = 95,
  VK_JIS_Eisu = 102,
  VK_JIS_Kana = 104,
} KeyCode;

typedef enum LogLevel {
  Off,
  Error,
  Warn,
  Info,
  Debug,
  Trace,
} LogLevel;

typedef enum WindowVisualEffect {
  TitlebarEffect,
  SelectionEffect,
  MenuEffect,
  PopoverEffect,
  SidebarEffect,
  HeaderViewEffect,
  SheetEffect,
  WindowBackgroundEffect,
  HUDWindowEffect,
  FullScreenUIEffect,
  ToolTipEffect,
  ContentBackgroundEffect,
  UnderWindowBackgroundEffect,
  UnderPageBackgroundEffect,
} WindowVisualEffect;

typedef struct DisplayLinkBox DisplayLinkBox;

typedef struct MetalView MetalView;

typedef struct Window Window;

typedef struct ApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} ApplicationConfig;

typedef int64_t WindowId;

typedef struct KeyModifiers {
  bool capslock;
  bool shift;
  bool control;
  bool option;
  bool command;
  bool numeric_pad;
  bool help;
  bool function;
} KeyModifiers;

typedef char *StrPtr;

typedef struct KeyDownEvent {
  WindowId window_id;
  struct KeyModifiers modifiers;
  enum KeyCode code;
  StrPtr characters;
  StrPtr key;
  bool is_repeat;
} KeyDownEvent;

typedef struct KeyUpEvent {
  WindowId window_id;
  struct KeyModifiers modifiers;
  enum KeyCode code;
  StrPtr characters;
  StrPtr key;
} KeyUpEvent;

typedef struct ModifiersChangedEvent {
  WindowId window_id;
  struct KeyModifiers modifiers;
  enum KeyCode code;
} ModifiersChangedEvent;

typedef double LogicalPixels;

typedef struct LogicalPoint {
  LogicalPixels x;
  LogicalPixels y;
} LogicalPoint;

typedef uint32_t MouseButtonsSet;

typedef struct MouseMovedEvent {
  WindowId window_id;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseMovedEvent;

typedef uint32_t MouseButton;

typedef struct MouseDraggedEvent {
  WindowId window_id;
  MouseButton button;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseDraggedEvent;

typedef struct MouseEnteredEvent {
  WindowId window_id;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseEnteredEvent;

typedef struct MouseExitedEvent {
  WindowId window_id;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseExitedEvent;

typedef struct MouseDownEvent {
  WindowId window_id;
  MouseButton button;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseDownEvent;

typedef struct MouseUpEvent {
  WindowId window_id;
  MouseButton button;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} MouseUpEvent;

typedef struct ScrollWheelEvent {
  WindowId window_id;
  LogicalPixels scrolling_delta_x;
  LogicalPixels scrolling_delta_y;
  bool has_precise_scrolling_deltas;
  struct LogicalPoint location_in_window;
  struct LogicalPoint location_in_screen;
  MouseButtonsSet pressed_buttons;
} ScrollWheelEvent;

typedef uint32_t ScreenId;

typedef struct WindowScreenChangeEvent {
  WindowId window_id;
  ScreenId new_screen_id;
} WindowScreenChangeEvent;

typedef struct LogicalSize {
  LogicalPixels width;
  LogicalPixels height;
} LogicalSize;

typedef struct WindowResizeEvent {
  WindowId window_id;
  struct LogicalSize size;
} WindowResizeEvent;

typedef struct WindowMoveEvent {
  WindowId window_id;
  struct LogicalPoint origin;
} WindowMoveEvent;

typedef struct WindowFocusChangeEvent {
  WindowId window_id;
  bool is_key;
  bool is_main;
} WindowFocusChangeEvent;

typedef struct WindowCloseRequestEvent {
  WindowId window_id;
} WindowCloseRequestEvent;

typedef struct WindowFullScreenToggleEvent {
  WindowId window_id;
  bool is_full_screen;
} WindowFullScreenToggleEvent;

typedef enum Event_Tag {
  KeyDown,
  KeyUp,
  ModifiersChanged,
  MouseMoved,
  MouseDragged,
  MouseEntered,
  MouseExited,
  MouseDown,
  MouseUp,
  ScrollWheel,
  WindowScreenChange,
  WindowResize,
  WindowMove,
  WindowFocusChange,
  WindowCloseRequest,
  WindowFullScreenToggle,
  DisplayConfigurationChange,
  ApplicationDidFinishLaunching,
} Event_Tag;

typedef struct Event {
  Event_Tag tag;
  union {
    struct {
      struct KeyDownEvent key_down;
    };
    struct {
      struct KeyUpEvent key_up;
    };
    struct {
      struct ModifiersChangedEvent modifiers_changed;
    };
    struct {
      struct MouseMovedEvent mouse_moved;
    };
    struct {
      struct MouseDraggedEvent mouse_dragged;
    };
    struct {
      struct MouseEnteredEvent mouse_entered;
    };
    struct {
      struct MouseExitedEvent mouse_exited;
    };
    struct {
      struct MouseDownEvent mouse_down;
    };
    struct {
      struct MouseUpEvent mouse_up;
    };
    struct {
      struct ScrollWheelEvent scroll_wheel;
    };
    struct {
      struct WindowScreenChangeEvent window_screen_change;
    };
    struct {
      struct WindowResizeEvent window_resize;
    };
    struct {
      struct WindowMoveEvent window_move;
    };
    struct {
      struct WindowFocusChangeEvent window_focus_change;
    };
    struct {
      struct WindowCloseRequestEvent window_close_request;
    };
    struct {
      struct WindowFullScreenToggleEvent window_full_screen_toggle;
    };
  };
} Event;

typedef bool (*EventHandler)(const struct Event*);

typedef struct ApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  EventHandler event_handler;
} ApplicationCallbacks;

typedef void *MetalDeviceRef;

typedef void *MetalCommandQueueRef;

typedef double PhysicalPixels;

typedef struct PhysicalSize {
  PhysicalPixels width;
  PhysicalPixels height;
} PhysicalSize;

typedef void *MetalTextureRef;

typedef void (*DisplayLinkCallback)(void);

typedef struct WindowParams {
  struct LogicalPoint origin;
  struct LogicalSize size;
  StrPtr title;
  bool is_resizable;
  bool is_closable;
  bool is_miniaturizable;
  bool is_full_screen_allowed;
  bool use_custom_titlebar;
  LogicalPixels titlebar_height;
} WindowParams;

typedef struct Color {
  double red;
  double green;
  double blue;
  double alpha;
} Color;

typedef enum WindowBackground_Tag {
  Transparent,
  SolidColor,
  VisualEffect,
} WindowBackground_Tag;

typedef struct WindowBackground {
  WindowBackground_Tag tag;
  union {
    struct {
      struct Color solid_color;
    };
    struct {
      enum WindowVisualEffect visual_effect;
    };
  };
} WindowBackground;

typedef struct ScreenInfo {
  ScreenId screen_id;
  bool is_primary;
  StrPtr name;
  struct LogicalPoint origin;
  struct LogicalSize size;
  double scale;
} ScreenInfo;

typedef int64_t ArraySize;

typedef struct ScreenInfoArray {
  struct ScreenInfo *ptr;
  ArraySize len;
} ScreenInfoArray;

typedef struct AppMenuKeystroke {
  StrPtr key;
  struct KeyModifiers modifiers;
} AppMenuKeystroke;

typedef enum AppMenuItem_Tag {
  ActionItem,
  SeparatorItem,
  SubMenuItem,
} AppMenuItem_Tag;

typedef struct ActionItem_Body {
  bool enabled;
  StrPtr title;
  bool macos_provided;
  const struct AppMenuKeystroke *keystroke;
  void (*perform)(void);
} ActionItem_Body;

typedef struct SubMenuItem_Body {
  StrPtr title;
  StrPtr special_tag;
  const struct AppMenuItem *items;
  ArraySize items_count;
} SubMenuItem_Body;

typedef struct AppMenuItem {
  AppMenuItem_Tag tag;
  union {
    ActionItem_Body action_item;
    SubMenuItem_Body sub_menu_item;
  };
} AppMenuItem;

typedef struct AppMenuStructure {
  const struct AppMenuItem *items;
  ArraySize items_count;
} AppMenuStructure;

typedef struct ExceptionsArray {
  const StrPtr *items;
  ArraySize count;
} ExceptionsArray;

typedef struct LoggerConfiguration {
  StrPtr file_path;
  enum LogLevel console_level;
  enum LogLevel file_level;
} LoggerConfiguration;

#define LeftMouseButton 0

#define RightMouseButton 1

#define MiddleMouseButton 2

bool dispatcher_is_main_thread(void);

void dispatcher_main_exec_async(void (*f)(void));

void application_init(const struct ApplicationConfig *config,
                      struct ApplicationCallbacks callbacks);

void application_shutdown(void);

void application_run_event_loop(void);

void application_stop_event_loop(void);

void application_request_termination(void);

MetalDeviceRef metal_create_device(void);

void metal_deref_device(MetalDeviceRef device);

MetalCommandQueueRef metal_create_command_queue(MetalDeviceRef device);

void metal_deref_command_queue(MetalCommandQueueRef queue);

struct MetalView *metal_create_view(MetalDeviceRef device);

void metal_drop_view(struct MetalView *view);

void metal_view_set_is_opaque(const struct MetalView *view, bool value);

bool metal_view_get_is_opaque(const struct MetalView *view);

void metal_view_present(const struct MetalView *view,
                        MetalCommandQueueRef queue,
                        bool wait_for_ca_transaction);

struct PhysicalSize metal_view_get_texture_size(const struct MetalView *view);

MetalTextureRef metal_view_next_texture(const struct MetalView *view);

void metal_deref_texture(MetalTextureRef texture);

struct DisplayLinkBox *display_link_create(ScreenId screen_id, DisplayLinkCallback on_next_frame);

void display_link_drop(struct DisplayLinkBox *display_link);

void display_link_set_running(struct DisplayLinkBox *display_link, bool value);

bool display_link_is_running(struct DisplayLinkBox *display_link);

struct Window *window_create(const struct WindowParams *params);

void window_drop(struct Window *window);

WindowId window_get_window_id(const struct Window *window);

ScreenId window_get_screen_id(const struct Window *window);

double window_scale_factor(const struct Window *window);

void window_attach_layer(const struct Window *window, const struct MetalView *layer);

void window_set_title(const struct Window *window, StrPtr new_title);

StrPtr window_get_title(const struct Window *window);

struct LogicalPoint window_get_origin(const struct Window *window);

struct LogicalSize window_get_size(const struct Window *window);

void window_set_rect(const struct Window *window,
                     struct LogicalPoint origin,
                     struct LogicalSize size,
                     bool animate);

bool window_is_key(const struct Window *window);

bool window_is_main(const struct Window *window);

struct LogicalSize window_get_max_size(const struct Window *window);

void window_set_max_size(const struct Window *window, struct LogicalSize size);

struct LogicalSize window_get_min_size(const struct Window *window);

void window_set_min_size(const struct Window *window, struct LogicalSize size);

void window_toggle_full_screen(const struct Window *window);

bool window_is_full_screen(const struct Window *window);

void window_start_drag(const struct Window *window);

void window_invalidate_shadow(const struct Window *window);

void window_set_background(const struct Window *window, struct WindowBackground background);

struct ScreenInfoArray screen_list(void);

void screen_list_drop(struct ScreenInfoArray arr);

ScreenId screen_get_main_screen_id(void);

void main_menu_update(struct AppMenuStructure menu);

void main_menu_set_none(void);

void string_drop(StrPtr str_ptr);

struct ExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct LoggerConfiguration *logger_configuration);
