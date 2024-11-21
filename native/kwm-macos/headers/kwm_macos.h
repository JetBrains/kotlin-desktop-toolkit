#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef const char *StrPtr;

typedef int64_t ArraySize;

typedef enum AppMenuItem_Tag {
  ActionItem,
  SeparatorItem,
  SubMenuItem,
} AppMenuItem_Tag;

typedef struct ActionItem_Body {
  bool enabled;
  StrPtr title;
} ActionItem_Body;

typedef struct SubMenuItem_Body {
  StrPtr title;
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

int32_t add_numbers(int32_t x, int32_t y);

void application_init(void);

void application_run_event_loop(void);

void main_menu_update(struct AppMenuStructure menu);
