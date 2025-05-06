package org.jetbrains.desktop.linux

public data class TextInputPreeditStringData(
    public val text: String?,
    public val cursorBeginBytePos: Int,
    public val cursorEndBytePos: Int,
) {
    internal companion object
}

public data class TextInputCommitStringData(
    public val text: String?,
) {
    internal companion object
}

public data class TextInputDeleteSurroundingTextData(
    public val beforeLengthInBytes: Int,
    public val afterLengthInBytes: Int,
) {
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

public class TextInputContext(
    public val surroundingText: String,
    public val cursorCodepointOffset: Short,
    public val selectionStartCodepointOffset: Short,
    public val isMultiline: Boolean,
    public val contentPurpose: TextInputContentPurpose,
    public val cursorRectangle: LogicalRect,
    public val changeCausedByInputMethod: Boolean,
) {
    internal companion object
}
