/* This header is generated please don't edit it manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct DisplayLink DisplayLink;

typedef struct DisplayLinkBox DisplayLinkBox;

typedef struct MetalView MetalView;

typedef struct Window Window;

typedef struct ApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} ApplicationConfig;

typedef int64_t WindowId;

typedef double LogicalPixels;

typedef struct LogicalPoint {
  LogicalPixels x;
  LogicalPixels y;
} LogicalPoint;

typedef struct MouseMovedEvent {
  WindowId window_id;
  struct LogicalPoint point;
} MouseMovedEvent;

typedef struct ScrollWheelEvent {
  WindowId window_id;
  LogicalPixels dx;
  LogicalPixels dy;
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
  MouseMoved,
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
      struct MouseMovedEvent mouse_moved;
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

typedef char *StrPtr;

typedef struct WindowParams {
  struct LogicalPoint origin;
  struct LogicalSize size;
  StrPtr title;
} WindowParams;

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

typedef uint32_t AppMenuKeyModifiers;
#define AppMenuKeyModifiers_ModifierFlagCapsLock (uint32_t)(1 << 16)
#define AppMenuKeyModifiers_ModifierFlagShift (uint32_t)(1 << 17)
#define AppMenuKeyModifiers_ModifierFlagControl (uint32_t)(1 << 18)
#define AppMenuKeyModifiers_ModifierFlagOption (uint32_t)(1 << 19)
#define AppMenuKeyModifiers_ModifierFlagCommand (uint32_t)(1 << 20)
#define AppMenuKeyModifiers_ModifierFlagNumericPad (uint32_t)(1 << 21)
#define AppMenuKeyModifiers_ModifierFlagHelp (uint32_t)(1 << 22)
#define AppMenuKeyModifiers_ModifierFlagFunction (uint32_t)(1 << 23)
#define AppMenuKeyModifiers_ModifierFlagDeviceIndependentFlagsMask (uint32_t)4294901760

typedef struct AppMenuKeystroke {
  StrPtr key;
  AppMenuKeyModifiers modifiers;
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

void metal_command_queue_commit(MetalCommandQueueRef queue);

void metal_deref_command_queue(MetalCommandQueueRef queue);

struct MetalView *metal_create_view(MetalDeviceRef device);

void metal_drop_view(struct MetalView *view);

void metal_view_present(const struct MetalView *view);

struct PhysicalSize metal_view_get_texture_size(const struct MetalView *view);

MetalTextureRef metal_view_next_texture(const struct MetalView *view);

void metal_deref_texture(MetalTextureRef texture);

struct DisplayLinkBox *display_link_create(ScreenId screen_id, DisplayLinkCallback on_next_frame);

void display_link_set_running(struct DisplayLink *display_link, bool value);

bool display_link_is_running(struct DisplayLink *display_link);

void display_link_drop(struct DisplayLink *display_link);

struct Window *window_create(struct WindowParams params);

void window_drop(struct Window *window);

WindowId window_get_window_id(const struct Window *window);

ScreenId window_get_screen_id(const struct Window *window);

double window_scale_factor(const struct Window *window);

void window_attach_layer(const struct Window *window, const struct MetalView *layer);

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

struct ScreenInfoArray screen_list(void);

void screen_list_drop(struct ScreenInfoArray arr);

void main_menu_update(struct AppMenuStructure menu);

void main_menu_set_none(void);
