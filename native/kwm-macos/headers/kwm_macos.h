/* This header is generated please don't edit it manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct DisplayLink DisplayLink;

typedef struct MetalView MetalView;

typedef struct ApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} ApplicationConfig;

typedef int64_t WindowId;

typedef struct Point {
  double x;
  double y;
} Point;

typedef struct MouseMovedEvent {
  WindowId window_id;
  struct Point point;
} MouseMovedEvent;

typedef struct ScrollWheelEvent {
  WindowId window_id;
  double dx;
  double dy;
} ScrollWheelEvent;

typedef enum Event_Tag {
  MouseMoved,
  ScrollWheel,
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

typedef struct Size {
  double width;
  double height;
} Size;

typedef void *MetalTextureRef;

typedef void *WindowRef;

typedef void (*DisplayLinkCallback)(void);

typedef const char *StrPtr;

typedef void (*WindowResizeCallback)(void);

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

typedef int64_t ArraySize;

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

struct Size metal_view_get_texture_size(const struct MetalView *view);

MetalTextureRef metal_view_next_texture(const struct MetalView *view);

void metal_deref_texture(MetalTextureRef texture);

struct DisplayLink *display_link_create(WindowRef window, DisplayLinkCallback on_next_frame);

void display_link_set_paused(const struct DisplayLink *display_link, bool value);

void display_link_drop(struct DisplayLink *display_link);

WindowRef window_create(StrPtr title, float x, float y, WindowResizeCallback on_resize);

void window_deref(WindowRef window);

WindowId window_get_window_id(WindowRef window);

void window_attach_layer(WindowRef window, const struct MetalView *layer);

void main_menu_update(struct AppMenuStructure menu);

void main_menu_set_none(void);
