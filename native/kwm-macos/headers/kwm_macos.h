/* This header is generated please don't edit it manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef const char *StrPtr;

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

typedef uint32_t SomeStruct;
#define SomeStruct_A (uint32_t)1
#define SomeStruct_B (uint32_t)2

typedef struct ApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} ApplicationConfig;

bool dispatcher_is_main_thread(void);

void dispatcher_main_exec_async(void (*f)(void));

void main_menu_update(struct AppMenuStructure menu);

void main_menu_set_none(void);

int32_t add_numbers(int32_t x, int32_t y, SomeStruct s);

void application_init(const struct ApplicationConfig *config);

void application_run_event_loop(void);

void application_create_window(StrPtr title, float x, float y);
