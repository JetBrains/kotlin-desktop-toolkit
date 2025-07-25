/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef enum NativeActionItemState {
  NativeActionItemState_On,
  NativeActionItemState_Off,
  NativeActionItemState_Mixed,
} NativeActionItemState;

typedef enum NativeActionMenuItemSpecialTag {
  NativeActionMenuItemSpecialTag_None,
  NativeActionMenuItemSpecialTag_Undo,
  NativeActionMenuItemSpecialTag_Redo,
  NativeActionMenuItemSpecialTag_Cut,
  NativeActionMenuItemSpecialTag_Copy,
  NativeActionMenuItemSpecialTag_Paste,
  NativeActionMenuItemSpecialTag_Delete,
} NativeActionMenuItemSpecialTag;

typedef enum NativeAppMenuTrigger {
  NativeAppMenuTrigger_Keystroke,
  NativeAppMenuTrigger_Other,
} NativeAppMenuTrigger;

typedef enum NativeAppearance {
  NativeAppearance_Dark,
  NativeAppearance_Light,
} NativeAppearance;

typedef enum NativeCursorIcon {
  NativeCursorIcon_Unknown,
  NativeCursorIcon_ArrowCursor,
  NativeCursorIcon_IBeamCursor,
  NativeCursorIcon_CrosshairCursor,
  NativeCursorIcon_ClosedHandCursor,
  NativeCursorIcon_OpenHandCursor,
  NativeCursorIcon_PointingHandCursor,
  NativeCursorIcon_ColumnResizeLeftCursor,
  NativeCursorIcon_ColumnResizeRightCursor,
  NativeCursorIcon_ColumnResizeLeftRightCursor,
  NativeCursorIcon_RowResizeUpCursor,
  NativeCursorIcon_RowResizeDownCursor,
  NativeCursorIcon_RowResizeUpDownCursor,
  NativeCursorIcon_FrameResizeUpLeftDownRight,
  NativeCursorIcon_FrameResizeUpRightDownLeft,
  NativeCursorIcon_DisappearingItemCursor,
  NativeCursorIcon_IBeamCursorForVerticalLayout,
  NativeCursorIcon_OperationNotAllowedCursor,
  NativeCursorIcon_DragLinkCursor,
  NativeCursorIcon_DragCopyCursor,
  NativeCursorIcon_ContextualMenuCursor,
  NativeCursorIcon_ZoomInCursor,
  NativeCursorIcon_ZoomOutCursor,
} NativeCursorIcon;

typedef enum NativeLogLevel {
  NativeLogLevel_Off,
  NativeLogLevel_Error,
  NativeLogLevel_Warn,
  NativeLogLevel_Info,
  NativeLogLevel_Debug,
  NativeLogLevel_Trace,
} NativeLogLevel;

typedef enum NativeSubMenuItemSpecialTag {
  NativeSubMenuItemSpecialTag_None,
  NativeSubMenuItemSpecialTag_AppNameMenu,
  NativeSubMenuItemSpecialTag_Window,
  NativeSubMenuItemSpecialTag_Services,
} NativeSubMenuItemSpecialTag;

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

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeRustAllocatedStrPtr;

typedef uintptr_t NativeArraySize;

typedef struct NativeExceptionsArray {
  const NativeRustAllocatedStrPtr *items;
  NativeArraySize count;
} NativeExceptionsArray;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

typedef struct NativeLoggerConfiguration {
  NativeBorrowedStrPtr file_path;
  enum NativeLogLevel console_level;
  enum NativeLogLevel file_level;
} NativeLoggerConfiguration;

typedef struct NativeApplicationConfig {
  bool disable_dictation_menu_item;
  bool disable_character_palette_menu_item;
} NativeApplicationConfig;

typedef intptr_t NativeWindowId;

typedef uintptr_t NativeKeyModifiersSet;

typedef uint16_t NativeKeyCode;

typedef double NativeTimestamp;

typedef struct NativeKeyDownEvent {
  NativeWindowId window_id;
  NativeKeyModifiersSet modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  NativeBorrowedStrPtr key_with_modifiers;
  bool is_repeat;
  bool might_have_key_equivalent;
  NativeTimestamp timestamp;
} NativeKeyDownEvent;

typedef struct NativeKeyUpEvent {
  NativeWindowId window_id;
  NativeKeyModifiersSet modifiers;
  NativeKeyCode code;
  NativeBorrowedStrPtr characters;
  NativeBorrowedStrPtr key;
  NativeBorrowedStrPtr key_with_modifiers;
  NativeTimestamp timestamp;
} NativeKeyUpEvent;

typedef struct NativeModifiersChangedEvent {
  NativeWindowId window_id;
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
  intptr_t click_count;
  NativeTimestamp timestamp;
} NativeMouseDownEvent;

typedef struct NativeMouseUpEvent {
  NativeWindowId window_id;
  NativeMouseButton button;
  struct NativeLogicalPoint location_in_window;
  intptr_t click_count;
  NativeTimestamp timestamp;
} NativeMouseUpEvent;

typedef struct NativeScrollWheelEvent {
  NativeWindowId window_id;
  NativeLogicalPixels scrolling_delta_x;
  NativeLogicalPixels scrolling_delta_y;
  bool has_precise_scrolling_deltas;
  bool is_direction_inverted;
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

typedef struct NativeWindowChangedOcclusionStateEvent {
  NativeWindowId window_id;
  bool is_visible;
} NativeWindowChangedOcclusionStateEvent;

typedef struct NativeAutoDropArray_RustAllocatedStrPtr {
  const NativeRustAllocatedStrPtr *ptr;
  NativeArraySize len;
} NativeAutoDropArray_RustAllocatedStrPtr;

typedef struct NativeApplicationOpenUrlsEvent {
  struct NativeAutoDropArray_RustAllocatedStrPtr urls;
} NativeApplicationOpenUrlsEvent;

typedef struct NativeApplicationAppearanceChangeEvent {
  enum NativeAppearance new_appearance;
} NativeApplicationAppearanceChangeEvent;

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
  NativeEvent_WindowChangedOcclusionState,
  NativeEvent_DisplayConfigurationChange,
  NativeEvent_ApplicationOpenUrls,
  NativeEvent_ApplicationDidFinishLaunching,
  NativeEvent_ApplicationAppearanceChange,
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
    struct {
      struct NativeWindowChangedOcclusionStateEvent window_changed_occlusion_state;
    };
    struct {
      struct NativeApplicationOpenUrlsEvent application_open_urls;
    };
    struct {
      struct NativeApplicationAppearanceChangeEvent application_appearance_change;
    };
  };
} NativeEvent;

typedef bool (*NativeEventHandler)(const struct NativeEvent*);

typedef struct NativeApplicationCallbacks {
  bool (*on_should_terminate)(void);
  void (*on_will_terminate)(void);
  NativeEventHandler event_handler;
} NativeApplicationCallbacks;

typedef struct NativeAppMenuKeystroke {
  NativeBorrowedStrPtr key;
  NativeKeyModifiersSet modifiers;
} NativeAppMenuKeystroke;

typedef void (*NativeAppMenuItemCallback)(enum NativeAppMenuTrigger trigger);

typedef enum NativeAppMenuItem_Tag {
  NativeAppMenuItem_ActionItem,
  NativeAppMenuItem_SeparatorItem,
  NativeAppMenuItem_SubMenuItem,
} NativeAppMenuItem_Tag;

typedef struct NativeAppMenuItem_NativeActionItem_Body {
  bool enabled;
  enum NativeActionItemState state;
  NativeBorrowedStrPtr title;
  enum NativeActionMenuItemSpecialTag special_tag;
  const struct NativeAppMenuKeystroke *keystroke;
  NativeAppMenuItemCallback perform;
} NativeAppMenuItem_NativeActionItem_Body;

typedef struct NativeAppMenuItem_NativeSubMenuItem_Body {
  NativeBorrowedStrPtr title;
  enum NativeSubMenuItemSpecialTag special_tag;
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

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeDisplayLinkPtr;

typedef void (*NativeDisplayLinkCallback)(void);

typedef uintptr_t NativeDragOperation;

typedef uintptr_t NativeDragOperationsBitSet;

typedef NativeRustAllocatedStrPtr NativeAutoDropStrPtr;

typedef struct NativeDragInfo {
  NativeWindowId destination_window_id;
  struct NativeLogicalPoint location_in_window;
  NativeDragOperationsBitSet allowed_operations;
  intptr_t sequence_number;
  NativeAutoDropStrPtr pasteboard_name;
} NativeDragInfo;

typedef NativeDragOperation (*NativeDragEnteredCallback)(struct NativeDragInfo info);

typedef NativeDragOperation (*NativeDragUpdatedCallback)(struct NativeDragInfo info);

typedef void (*NativeDragExitedCallback)(NativeRustAllocatedRawPtr info);

typedef bool (*NativeDragPerformCallback)(struct NativeDragInfo info);

typedef struct NativeDragAndDropCallbacks {
  NativeDragEnteredCallback drag_entered_callback;
  NativeDragUpdatedCallback drag_updated_callback;
  NativeDragExitedCallback drag_exited_callback;
  NativeDragPerformCallback drag_perform_callback;
} NativeDragAndDropCallbacks;

typedef uint32_t NativeMouseButtonsSet;

typedef struct NativeCommonFileDialogParams {
  NativeBorrowedStrPtr title;
  NativeBorrowedStrPtr prompt;
  NativeBorrowedStrPtr message;
  NativeBorrowedStrPtr name_field_label;
  NativeBorrowedStrPtr name_field_string_value;
  NativeBorrowedStrPtr directory_url;
  bool can_create_directories;
  bool can_select_hidden_extension;
  bool shows_hidden_files;
  bool extensions_hidden;
} NativeCommonFileDialogParams;

typedef struct NativeOpenFileDialogParams {
  bool can_choose_files;
  bool can_choose_directories;
  bool resolves_aliases;
  bool allows_multiple_selection;
} NativeOpenFileDialogParams;

typedef void *NativeMetalDeviceRef;

typedef void *NativeMetalCommandQueueRef;

typedef NativeRustAllocatedRawPtr NativeMetalViewPtr;

typedef void (*NativeOnDisplayLayerCallback)(void);

typedef double NativePhysicalPixels;

typedef struct NativePhysicalSize {
  NativePhysicalPixels width;
  NativePhysicalPixels height;
} NativePhysicalSize;

typedef void *NativeMetalTextureRef;

typedef struct NativeCombinedItemElement {
  NativeBorrowedStrPtr uniform_type_identifier;
  NativeBorrowedStrPtr content;
} NativeCombinedItemElement;

typedef struct NativeBorrowedArray_CombinedItemElement {
  const struct NativeCombinedItemElement *ptr;
  NativeArraySize len;
  void (*deinit)(const struct NativeCombinedItemElement*, NativeArraySize);
} NativeBorrowedArray_CombinedItemElement;

typedef enum NativePasteboardItem_Tag {
  NativePasteboardItem_URLItem,
  NativePasteboardItem_CombinedItem,
} NativePasteboardItem_Tag;

typedef struct NativePasteboardItem_NativeURLItem_Body {
  NativeBorrowedStrPtr url;
} NativePasteboardItem_NativeURLItem_Body;

typedef struct NativePasteboardItem_NativeCombinedItem_Body {
  struct NativeBorrowedArray_CombinedItemElement elements;
} NativePasteboardItem_NativeCombinedItem_Body;

typedef struct NativePasteboardItem {
  NativePasteboardItem_Tag tag;
  union {
    NativePasteboardItem_NativeURLItem_Body url_item;
    NativePasteboardItem_NativeCombinedItem_Body combined_item;
  };
} NativePasteboardItem;

typedef struct NativeBorrowedArray_PasteboardItem {
  const struct NativePasteboardItem *ptr;
  NativeArraySize len;
  void (*deinit)(const struct NativePasteboardItem*, NativeArraySize);
} NativeBorrowedArray_PasteboardItem;

typedef struct NativePasteboardContentResult {
  struct NativeAutoDropArray_RustAllocatedStrPtr items;
} NativePasteboardContentResult;

typedef struct NativeScreenInfo {
  NativeScreenId screen_id;
  bool is_primary;
  NativeAutoDropStrPtr name;
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
  double scale;
  uint32_t maximum_frames_per_second;
} NativeScreenInfo;

typedef struct NativeAutoDropArray_ScreenInfo {
  const struct NativeScreenInfo *ptr;
  NativeArraySize len;
} NativeAutoDropArray_ScreenInfo;

typedef struct NativeAutoDropArray_ScreenInfo NativeScreenInfoArray;

typedef NativeRustAllocatedRawPtr NativeWindowPtr;

typedef struct NativeWindowParams {
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
  NativeBorrowedStrPtr title;
  bool is_resizable;
  bool is_closable;
  bool is_miniaturizable;
  bool is_full_screen_allowed;
  bool use_custom_titlebar;
  NativeLogicalPixels titlebar_height;
} NativeWindowParams;

typedef bool (*NativeHasMarkedTextCallback)(void);

typedef struct NativeTextRange {
  uintptr_t location;
  uintptr_t length;
} NativeTextRange;

typedef void (*NativeMarkedRangeCallback)(struct NativeTextRange *range_out);

typedef void (*NativeSelectedRangeCallback)(struct NativeTextRange *range_out);

typedef struct NativeInsertTextArgs {
  NativeBorrowedStrPtr text;
  struct NativeTextRange replacement_range;
} NativeInsertTextArgs;

typedef void (*NativeInsertTextCallback)(struct NativeInsertTextArgs args);

typedef bool (*NativeDoCommandCallback)(NativeBorrowedStrPtr command);

typedef void (*NativeUnmarkTextCallback)(void);

typedef struct NativeSetMarkedTextArgs {
  NativeBorrowedStrPtr text;
  struct NativeTextRange selected_range;
  struct NativeTextRange replacement_range;
} NativeSetMarkedTextArgs;

typedef void (*NativeSetMarkedTextCallback)(struct NativeSetMarkedTextArgs args);

typedef struct NativeAttributedStringForRangeResult {
  NativeBorrowedStrPtr string;
  struct NativeTextRange actual_range;
} NativeAttributedStringForRangeResult;

typedef struct NativeAttributedStringForRangeResult (*NativeAttributedStringForRangeCallback)(struct NativeTextRange range);

typedef void (*NativeFreeAttributedStringCallback)(void);

typedef struct NativeLogicalRect {
  struct NativeLogicalPoint origin;
  struct NativeLogicalSize size;
} NativeLogicalRect;

typedef struct NativeFirstRectForCharacterRangeArgs {
  struct NativeTextRange range_in;
  struct NativeTextRange actual_range_out;
  struct NativeLogicalRect first_rect_out;
} NativeFirstRectForCharacterRangeArgs;

typedef void (*NativeFirstRectForCharacterRangeCallback)(struct NativeFirstRectForCharacterRangeArgs *args);

typedef uintptr_t (*NativeCharacterIndexForPoint)(struct NativeLogicalPoint);

typedef struct NativeTextInputClient {
  NativeHasMarkedTextCallback has_marked_text;
  NativeMarkedRangeCallback marked_range;
  NativeSelectedRangeCallback selected_range;
  NativeInsertTextCallback insert_text;
  NativeDoCommandCallback do_command;
  NativeUnmarkTextCallback unmark_text;
  NativeSetMarkedTextCallback set_marked_text;
  NativeAttributedStringForRangeCallback attributed_string_for_range;
  NativeFreeAttributedStringCallback free_attributed_string_for_range;
  NativeFirstRectForCharacterRangeCallback first_rect_for_character_range;
  NativeCharacterIndexForPoint character_index_for_point;
} NativeTextInputClient;

typedef struct NativeBorrowedArray_BorrowedStrPtr {
  const NativeBorrowedStrPtr *ptr;
  NativeArraySize len;
  void (*deinit)(const NativeBorrowedStrPtr*, NativeArraySize);
} NativeBorrowedArray_BorrowedStrPtr;

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



struct NativeExceptionsArray logger_check_exceptions(void);

void logger_clear_exceptions(void);

void logger_init(const struct NativeLoggerConfiguration *logger_configuration);

void application_init(const struct NativeApplicationConfig *config,
                      struct NativeApplicationCallbacks callbacks);

enum NativeAppearance application_get_appearance(void);

void application_shutdown(void);

void application_run_event_loop(void);

void application_stop_event_loop(void);

void application_request_termination(void);

NativeRustAllocatedStrPtr application_get_name(void);

void application_hide(void);

void application_hide_other_applications(void);

void application_unhide_all_applications(void);

bool application_is_active(void);

void application_activate_ignoring_other_apps(void);

/**
 * # Safety
 *
 * `data` must be a valid, non-null, pointer.
 */
void application_set_dock_icon(uint8_t *data, uint64_t data_length);

void application_order_front_character_palete(void);

bool application_open_url(NativeBorrowedStrPtr url);

void main_menu_update(struct NativeAppMenuStructure menu);

void main_menu_set_none(void);

bool main_menu_offer_current_event(void);

void cursor_push_hide(void);

void cursor_pop_hide(void);

void cursor_set_icon(enum NativeCursorIcon icon);

enum NativeCursorIcon cursor_get_icon(void);

bool dispatcher_is_main_thread(void);

void dispatcher_main_exec_async(void (*f)(void));

intptr_t push_autorelease_pool(void);

void pop_autorelease_pool(intptr_t pool_ptr);

void set_qos_for_current_thread(intptr_t qos);

NativeDisplayLinkPtr display_link_create(NativeScreenId screen_id,
                                         NativeDisplayLinkCallback on_next_frame);

void display_link_drop(NativeDisplayLinkPtr display_link_ptr);

void display_link_set_running(NativeDisplayLinkPtr display_link_ptr, bool value);

bool display_link_is_running(NativeDisplayLinkPtr display_link_ptr);

void set_drag_and_drop_callbacks(struct NativeDragAndDropCallbacks callbacks);

void drop_drag_and_drop_callbacks(void);

NativeMouseButtonsSet events_pressed_mouse_buttons(void);

NativeKeyModifiersSet events_pressed_modifiers(void);

struct NativeLogicalPoint events_cursor_location_in_screen(void);

struct NativeAutoDropArray_RustAllocatedStrPtr open_file_dialog_run_modal(const struct NativeCommonFileDialogParams *common_params,
                                                                          const struct NativeOpenFileDialogParams *params);

NativeRustAllocatedStrPtr save_file_dialog_run_modal(const struct NativeCommonFileDialogParams *common_params);

NativeMetalDeviceRef metal_create_device(void);

void metal_deref_device(NativeMetalDeviceRef device);

NativeMetalCommandQueueRef metal_create_command_queue(NativeMetalDeviceRef device);

void metal_deref_command_queue(NativeMetalCommandQueueRef queue);

NativeMetalViewPtr metal_create_view(NativeMetalDeviceRef device,
                                     NativeOnDisplayLayerCallback on_display_layer);

void metal_drop_view(NativeMetalViewPtr view_ptr);

void metal_view_set_needs_display(NativeMetalViewPtr view_ptr);

void metal_view_set_is_opaque(NativeMetalViewPtr view_ptr, bool value);

bool metal_view_get_is_opaque(NativeMetalViewPtr view_ptr);

void metal_view_present(NativeMetalViewPtr view_ptr,
                        NativeMetalCommandQueueRef queue,
                        bool wait_for_ca_transaction);

struct NativePhysicalSize metal_view_get_texture_size(NativeMetalViewPtr view_ptr);

NativeMetalTextureRef metal_view_next_texture(NativeMetalViewPtr view_ptr);

void metal_deref_texture(NativeMetalTextureRef texture);

intptr_t pasteboard_clear(void);

bool pasteboard_write_objects(struct NativeBorrowedArray_PasteboardItem items);

struct NativePasteboardContentResult pasteboard_read_items_of_type(NativeBorrowedStrPtr pasteboard_name,
                                                                   NativeBorrowedStrPtr uniform_type_identifier);

struct NativePasteboardContentResult pasteboard_read_file_items(NativeBorrowedStrPtr pasteboard_name);

void pasteboard_content_drop(struct NativePasteboardContentResult content);

NativeScreenInfoArray screen_list(void);

void screen_list_drop(NativeScreenInfoArray arr);

NativeScreenId screen_get_main_screen_id(void);

void string_drop(NativeRustAllocatedStrPtr str_ptr);

void string_array_drop(struct NativeAutoDropArray_RustAllocatedStrPtr str_array);

bool text_input_context_handle_current_event(NativeWindowPtr window_ptr);

void text_input_context_discard_marked_text(NativeWindowPtr window_ptr);

void text_input_context_invalidate_character_coordinates(NativeWindowPtr window_ptr);

intptr_t text_input_context_not_found_offset(void);

void text_input_context_beep(void);

NativeWindowPtr window_create(struct NativeWindowParams params,
                              struct NativeTextInputClient text_input_client);

void window_drop(NativeWindowPtr window_ptr);

NativeWindowId window_get_window_id(NativeWindowPtr window_ptr);

NativeScreenId window_get_screen_id(NativeWindowPtr window_ptr);

double window_scale_factor(NativeWindowPtr window_ptr);

void window_attach_layer(NativeWindowPtr window_ptr, NativeMetalViewPtr layer_ptr);

void window_set_title(NativeWindowPtr window_ptr, NativeBorrowedStrPtr new_title);

NativeRustAllocatedStrPtr window_get_title(NativeWindowPtr window_ptr);

struct NativeLogicalPoint window_get_origin(NativeWindowPtr window_ptr);

struct NativeLogicalSize window_get_size(NativeWindowPtr window_ptr);

void window_set_rect(NativeWindowPtr window_ptr,
                     struct NativeLogicalPoint origin,
                     struct NativeLogicalSize size,
                     bool animate);

struct NativeLogicalPoint window_get_content_origin(NativeWindowPtr window_ptr);

struct NativeLogicalSize window_get_content_size(NativeWindowPtr window_ptr);

void window_set_content_rect(NativeWindowPtr window_ptr,
                             struct NativeLogicalPoint origin,
                             struct NativeLogicalSize size,
                             bool animate);

bool window_is_key(NativeWindowPtr window_ptr);

bool window_is_main(NativeWindowPtr window_ptr);

void window_order_front(NativeWindowPtr window_ptr);

void window_order_back(NativeWindowPtr window_ptr);

void window_make_key_and_order_front(NativeWindowPtr window_ptr);

struct NativeLogicalSize window_get_max_size(NativeWindowPtr window_ptr);

void window_set_max_size(NativeWindowPtr window_ptr, struct NativeLogicalSize size);

struct NativeLogicalSize window_get_min_size(NativeWindowPtr window_ptr);

void window_set_min_size(NativeWindowPtr window_ptr, struct NativeLogicalSize size);

void window_toggle_full_screen(NativeWindowPtr window_ptr);

bool window_is_full_screen(NativeWindowPtr window_ptr);

void window_maximize(NativeWindowPtr window_ptr);

bool window_is_maximized(NativeWindowPtr window_ptr);

void window_miniaturize(NativeWindowPtr window_ptr);

void window_deminiaturize(NativeWindowPtr window_ptr);

bool window_is_miniaturized(NativeWindowPtr window_ptr);

bool window_is_visible(NativeWindowPtr window_ptr);

void window_start_drag(NativeWindowPtr window_ptr);

void window_invalidate_shadow(NativeWindowPtr window_ptr);

void window_appearance_override(NativeWindowPtr window_ptr, enum NativeAppearance appearance);

bool window_appearance_is_overridden(NativeWindowPtr window_ptr);

void window_appearance_set_follow_application(NativeWindowPtr window_ptr);

enum NativeAppearance window_get_appearance(NativeWindowPtr window_ptr);

void window_register_for_dragged_types(NativeWindowPtr window_ptr,
                                       struct NativeBorrowedArray_BorrowedStrPtr types);

void window_unregister_dragged_types(NativeWindowPtr window_ptr);

void window_set_background(NativeWindowPtr window_ptr, struct NativeWindowBackground background);
