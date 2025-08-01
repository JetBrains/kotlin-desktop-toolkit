/* This header is generated please don't edit it manually. */

#include <stdbool.h>
#include <stdint.h>

typedef const void *NativeGenericRawPtr_c_void;

typedef NativeGenericRawPtr_c_void NativeRustAllocatedRawPtr;

typedef NativeRustAllocatedRawPtr NativeAppPtr;

typedef NativeRustAllocatedRawPtr NativeWindowPtr;

typedef int32_t NativePhysicalPixels;

typedef struct NativePhysicalPoint {
  NativePhysicalPixels x;
  NativePhysicalPixels y;
} NativePhysicalPoint;

typedef struct NativePhysicalSize {
  NativePhysicalPixels width;
  NativePhysicalPixels height;
} NativePhysicalSize;

typedef const char *NativeGenericRawPtr_c_char;

typedef NativeGenericRawPtr_c_char NativeBorrowedStrPtr;

typedef struct NativeWindowParams {
  struct NativePhysicalPoint origin;
  struct NativePhysicalSize size;
  NativeBorrowedStrPtr title;
  bool is_resizable;
  bool is_closable;
  bool is_minimizable;
} NativeWindowParams;

NativeAppPtr application_init(void);

void application_run_event_loop(void);

void application_stop_event_loop(void);

NativeWindowPtr window_create(NativeAppPtr app_ptr, struct NativeWindowParams params);

void window_drop(NativeWindowPtr window_ptr);

void window_show(NativeWindowPtr window_ptr);
