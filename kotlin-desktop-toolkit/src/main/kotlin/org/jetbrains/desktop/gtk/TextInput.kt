package org.jetbrains.desktop.gtk

public enum class TextInputPreeditUnderlineType {
    None,
    Single,
    Double,
    Low,
    Error,
    ;

    internal companion object
}

@ConsistentCopyVisibility
public data class TextInputPreeditAttribute internal constructor(
    val beginBytePos: UInt,
    val endBytePos: UInt,
    val underline: TextInputPreeditUnderlineType,
    val foregroundHighlight: Boolean,
    val backgroundHighlight: Boolean,
    val strikethrough: Boolean,
    val bold: Boolean,
    val italic: Boolean,
) {
    internal companion object
}

@ConsistentCopyVisibility
public data class TextInputPreeditStringData internal constructor(
    public val text: String?,
    public val cursorBytePos: Int,
    public val attributes: List<TextInputPreeditAttribute>,
) {
    internal companion object
}

@ConsistentCopyVisibility
public data class TextInputCommitStringData internal constructor(public val text: String?) {
    internal companion object
}

@ConsistentCopyVisibility
public data class TextInputDeleteSurroundingTextData internal constructor(
    public val beforeLengthInBytes: UInt,
    public val afterLengthInBytes: UInt,
) {
    internal companion object
}

public enum class TextInputContextHint {
    Spellcheck,
    NoSpellcheck,
    WordCompletion,
    Lowercase,
    UppercaseChars,
    UppercaseWords,
    UppercaseSentences,
    InhibitOsk,
    VerticalWriting,
    Emoji,
    NoEmoji,
    Private,
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

public data class TextInputContext(
    public val hints: Set<TextInputContextHint>,
    public val contentPurpose: TextInputContentPurpose,
    public val cursorRectangle: LogicalRect,
) {
    internal companion object
}

public data class TextInputSurroundingText(
    public val surroundingText: String,
    public val cursorCodepointOffset: UShort,
    public val selectionStartCodepointOffset: UShort,
) {
    internal companion object
}
