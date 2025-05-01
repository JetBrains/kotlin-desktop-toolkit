package org.jetbrains.desktop.linux

public class TextInputPreeditStringData(public val text: ByteArray?, public val cursorBeginBytePos: Int, public val cursorEndBytePos: Int) {
    internal companion object
}

public class TextInputCommitStringData(public val text: ByteArray?) {
    internal companion object
}

public data class TextInputDeleteSurroundingTextData(public val beforeLengthInBytes: Int, public val afterLengthInBytes: Int) {
    internal companion object
}

public enum class TextInputContentPurpose {
    /** default input, allowing all characters */
    Normal,

    /** allow only alphabetic characters */
    Alpha,

    /** allow only digits */
    Digits,

    /** input a number (including decimal separator and sign) */
    Number,

    /** input a phone number */
    Phone,

    /** input an URL */
    Url,

    /** input an email address */
    Email,

    /** input a name of a person */
    Name,

    /** input a password (combine with sensitive_data hint) */
    Password,

    /** input is a numeric password (combine with sensitive_data hint) */
    Pin,

    /** input a date */
    Date,
    Time,
    Datetime,
    Terminal,
    ;

    internal companion object
}

public data class TextInputContext(
    public val surroundingText: String,
    public val cursorPosBytes: Int,
    public val selectionStartPosBytes: Int,
    public val isMultiline: Boolean,
    public val contentPurpose: TextInputContentPurpose,
    public val cursorRectangle: LogicalRect,
    public val changeCausedByInputMethod: Boolean,
) {
    internal companion object
}
