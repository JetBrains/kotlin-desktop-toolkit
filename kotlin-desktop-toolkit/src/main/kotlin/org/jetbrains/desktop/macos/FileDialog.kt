package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeFileDialogParams
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object FileDialog {
    public data class DialogParams(
        val allowFile: Boolean = true,
        val allowFolder: Boolean = true,
        val allowMultipleSelection: Boolean = false,
    )

    public fun showModal(params: DialogParams): String? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val nativeParams = NativeFileDialogParams.allocate(arena)
                NativeFileDialogParams.allow_file(nativeParams, params.allowFile)
                NativeFileDialogParams.allow_folder(nativeParams, params.allowFolder)
                NativeFileDialogParams.allow_multiple_selection(nativeParams, params.allowMultipleSelection)

                val result = desktop_macos_h.file_dialog_run_modal(nativeParams)
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
}