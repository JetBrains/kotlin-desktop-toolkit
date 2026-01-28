package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeFileDialogOptions
import org.jetbrains.desktop.win32.generated.NativeFileOpenDialogOptions
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object FileDialog {
    public data class FileDialogOptions(
        val title: String? = null,
        val prompt: String? = null,
        val nameFieldLabel: String? = null,
        val nameFieldStringValue: String? = null,
        val directoryPath: String? = null,

        val showsHiddenFiles: Boolean = false,
    )

    public data class FileOpenDialogOptions(
        val chooseDirectories: Boolean = false,
        val allowsMultipleSelection: Boolean = false,
    )

    public fun showSaveFileDialog(owner: Window, options: FileDialogOptions = FileDialogOptions()): String? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val nativeCommonDialogParams = options.toNative(arena)
                val result = owner.withPointer { windowPtr ->
                    desktop_win32_h.save_file_dialog_run_modal(windowPtr, nativeCommonDialogParams)
                }
                if (result != MemorySegment.NULL) {
                    try {
                        result.getUtf8String(0).takeUnless { it.isEmpty() }
                    } finally {
                        ffiDownCall { desktop_win32_h.native_string_drop(result) }
                    }
                } else {
                    null
                }
            }
        }
    }

    public fun showOpenFileDialog(
        owner: Window,
        options: FileDialogOptions = FileDialogOptions(),
        openDialogOptions: FileOpenDialogOptions = FileOpenDialogOptions(),
    ): List<String> {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val nativeCommonDialogParams = options.toNative(arena)
                val nativeOpenFileDialogParams = openDialogOptions.toNative(arena)
                val result = owner.withPointer { windowPtr ->
                    desktop_win32_h.open_file_dialog_run_modal(arena, windowPtr, nativeCommonDialogParams, nativeOpenFileDialogParams)
                }
                if (result != MemorySegment.NULL) {
                    try {
                        listOfStringsFromNative(result)
                    } finally {
                        ffiDownCall { desktop_win32_h.native_string_array_drop(result) }
                    }
                } else {
                    emptyList()
                }
            }
        }
    }

    internal fun FileDialogOptions.toNative(arena: Arena): MemorySegment {
        val result = NativeFileDialogOptions.allocate(arena)
        NativeFileDialogOptions.title(result, title?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeFileDialogOptions.prompt(result, prompt?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeFileDialogOptions.name_field_label(result, nameFieldLabel?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeFileDialogOptions.name_field_string_value(
            result,
            nameFieldStringValue?.let {
                arena.allocateUtf8String(it)
            } ?: MemorySegment.NULL,
        )
        NativeFileDialogOptions.directory_path(result, directoryPath?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeFileDialogOptions.shows_hidden_files(result, showsHiddenFiles)
        return result
    }

    internal fun FileOpenDialogOptions.toNative(arena: Arena): MemorySegment {
        val result = NativeFileOpenDialogOptions.allocate(arena)
        NativeFileOpenDialogOptions.choose_directories(result, chooseDirectories)
        NativeFileOpenDialogOptions.allows_multiple_selection(result, allowsMultipleSelection)
        return result
    }
}
