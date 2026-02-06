package org.jetbrains.desktop.gtk

public data class TextInputPreeditStringData(
    public val text: String?,
    public val cursorBeginBytePos: Int,
    public val cursorEndBytePos: Int,
) {
    internal companion object
}

public data class TextInputCommitStringData(public val text: String?) {
    internal companion object
}

public data class TextInputDeleteSurroundingTextData(
    public val beforeLengthInBytes: UInt,
    public val afterLengthInBytes: UInt,
) {
    internal companion object
}

public enum class TextInputContextHint {
    WordCompletion,
    Spellcheck,
    Lowercase,
    UppercaseChars,
    UppercaseWords,
    UppercaseSentences,
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

    /** input a password */
    Password,

    /** input is a numeric password */
    Pin,

    Terminal,
    ;

    internal companion object
}

public class TextInputContext(
    public val surroundingText: String,
    public val cursorCodepointOffset: UShort,
    public val selectionStartCodepointOffset: UShort,
    public val hints: Set<TextInputContextHint>,
    public val contentPurpose: TextInputContentPurpose,
    public val cursorRectangle: LogicalRect,
) {
    internal companion object
}
