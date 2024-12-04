/* This header is generated please don't edit it manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef const char *StrPtr;

typedef Internal AppMenuKeyModifiers;

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

typedef struct ApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} ApplicationConfig;

typedef int32_t WindowId;

typedef void *MetalDeviceRef;

typedef void *MetalQueueRef;

bool dispatcher_is_main_thread(void);

void dispatcher_main_exec_async(void (*f)(void));

void main_menu_update(struct AppMenuStructure menu);

void main_menu_set_none(void);

void application_init(const struct ApplicationConfig *config);

void application_run_event_loop(void);

WindowId application_create_window(StrPtr title, float x, float y);

MetalDeviceRef metal_create_device(void);

void metal_deref_device(MetalDeviceRef device_ref);

MetalQueueRef metal_create_command_queue(MetalDeviceRef device_ref);

void metal_deref_command_queue(MetalQueueRef queue_ref);

void metal_create_layer(void);

void metal_layer_attach_to_window(void);
