package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.macos.generated.NativeOpenFileDialogParams
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object FileDialog {
    public data class CommonDialogParams(
        val title: String? = null,
        val prompt: String? = null,
        val message: String? = null,
        val nameFieldLabel: String? = null,
        val nameFieldStringValue: String? = null,
        val directoryUrl: String? = null,

        val canCreateDirectories: Boolean = false,
        val canSelectHiddenExtensions: Boolean = false,
        val showsHiddenFiles: Boolean = false,
        val extensionsHidden: Boolean = true,
    )

    public data class OpenDialogParams(
        val canChooseFiles: Boolean = true,
        val canChooseDirectories: Boolean = true,
        val resolveAliases: Boolean = true,
        val allowsMultipleSelections: Boolean = false,

    )

    public fun showSaveFileDialog(params: CommonDialogParams = CommonDialogParams()): String? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val nativeCommonDialogParams = params.toNative(arena)
                val result = desktop_macos_h.save_file_dialog_run_modal(nativeCommonDialogParams)
                if (result != MemorySegment.NULL) {
                    try {
                        result.getUtf8String(0)
                    } finally {
                        ffiDownCall { desktop_macos_h.string_drop(result) }
                    }
                } else {
                    null
                }
            }
        }
    }

    public fun showOpenFileDialog(
        params: CommonDialogParams = CommonDialogParams(),
        openDialogParams: OpenDialogParams = OpenDialogParams(),
    ): List<String> {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val nativeCommonDialogParams = params.toNative(arena)
                val nativeOpenFileDialogParams = openDialogParams.toNative(arena)
                val result = desktop_macos_h.open_file_dialog_run_modal(arena, nativeCommonDialogParams, nativeOpenFileDialogParams)
                if (result != MemorySegment.NULL) {
                    try {
                        listOfStringsFromNative(result)
                    } finally {
                        ffiDownCall { desktop_macos_h.string_array_drop(result) }
                    }
                } else {
                    emptyList()
                }
            }
        }
    }

    internal fun CommonDialogParams.toNative(arena: Arena): MemorySegment {
        val result = NativeCommonFileDialogParams.allocate(arena)
        NativeCommonFileDialogParams.title(result, title?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeCommonFileDialogParams.prompt(result, prompt?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeCommonFileDialogParams.message(result, message?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeCommonFileDialogParams.name_field_label(result, nameFieldLabel?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeCommonFileDialogParams.name_field_string_value(
            result,
            nameFieldStringValue?.let {
                arena.allocateUtf8String(it)
            } ?: MemorySegment.NULL,
        )
        NativeCommonFileDialogParams.directory_url(result, directoryUrl?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
        NativeCommonFileDialogParams.can_create_directories(result, canCreateDirectories)
        NativeCommonFileDialogParams.can_select_hidden_extension(result, canSelectHiddenExtensions)
        NativeCommonFileDialogParams.shows_hidden_files(result, showsHiddenFiles)
        NativeCommonFileDialogParams.extensions_hidden(result, extensionsHidden)
        return result
    }

    internal fun OpenDialogParams.toNative(arena: Arena): MemorySegment {
        val result = NativeOpenFileDialogParams.allocate(arena)
        NativeOpenFileDialogParams.can_choose_files(result, canChooseFiles)
        NativeOpenFileDialogParams.can_choose_directories(result, canChooseDirectories)
        NativeOpenFileDialogParams.resolves_aliases(result, resolveAliases)
        NativeOpenFileDialogParams.allows_multiple_selection(result, allowsMultipleSelections)
        return result
    }
}
