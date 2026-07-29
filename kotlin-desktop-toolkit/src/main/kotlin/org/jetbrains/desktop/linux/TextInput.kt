package org.jetbrains.desktop.linux

public class TextInputPreeditStringData internal constructor(
    public val text: String?,
    public val cursorBeginBytePos: Int,
    public val cursorEndBytePos: Int,
) {
    internal companion object;

    override fun toString(): String {
        return "TextInputPreeditStringData(text=$text, cursorBeginBytePos=$cursorBeginBytePos, cursorEndBytePos=$cursorEndBytePos)"
    }
}

public class TextInputCommitStringData internal constructor(public val text: String?) {
    internal companion object;

    override fun toString(): String {
        return "TextInputCommitStringData(text=$text)"
    }
}

public class TextInputDeleteSurroundingTextData internal constructor(
    public val beforeLengthInBytes: UInt,
    public val afterLengthInBytes: UInt,
) {
    internal companion object;

    override fun toString(): String {
        return "TextInputDeleteSurroundingTextData(beforeLengthInBytes=$beforeLengthInBytes, afterLengthInBytes=$afterLengthInBytes)"
    }
}

public enum class TextInputContentHint {
    Completion,
    Spellcheck,
    AutoCapitalization,
    Lowercase,
    Uppercase,
    Titlecase,
    HiddenText,
    SensitiveData,
    Latin,
    Multiline,
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
    public val cursorCodepointOffset: UShort,
    public val selectionStartCodepointOffset: UShort,
    public val hints: Set<TextInputContentHint>,
    public val contentPurpose: TextInputContentPurpose,
    public val cursorRectangle: LogicalRect,
    public val changeCausedByInputMethod: Boolean,
) {
    internal companion object
}
