package org.jetbrains.desktop.gtk

public object FileDialog {
    public data class CommonDialogParams(
        val modal: Boolean,
        val title: String?,
        val acceptLabel: String?,
        val currentFolder: String?,
    )

    public data class OpenDialogParams(
        val selectDirectories: Boolean,
        val allowsMultipleSelections: Boolean,
    )

    public data class SaveDialogParams(val nameFieldStringValue: String?)
}
