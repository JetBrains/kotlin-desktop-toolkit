package org.jetbrains.desktop.sample.compose

fun clipboardEntry(vararg items: ClipboardItem): ClipboardEntry = ClipboardItemsEntry(items.toList())

fun clipboardEntry(block: ClipboardEntryBuilderScope.() -> Unit): ClipboardEntry {
    val builder = ClipboardEntryBuilderImpl().apply(block)
    return clipboardEntry(*builder.build().toTypedArray())
}

interface ClipboardEntry {
    suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T>
    fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T>
    fun <T : Any> getFirstOrNullForFormatSync(format: ClipboardFormat<T>): T? = getForFormatSync(format).firstOrNull()

    suspend fun <T : Any> getFirstOrNullForFormat(format: ClipboardFormat<T>): T? = getForFormat(format).firstOrNull()
}

internal class ClipboardItemsEntry(val items: List<ClipboardItem>) : ClipboardEntry {
    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> = getForFormatSync(format)

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> {
        @Suppress("UNCHECKED_CAST")
        return items.flatMap { it.elements }
            .filter { it.format == format }
            .map { it.value } as List<T>
    }
}

sealed interface ClipboardFormat<T : Any> {
    data object Utf8PlainText : ClipboardFormat<String>
    data object Html : ClipboardFormat<String>
    data object Png : ClipboardFormat<ByteArray>
    data object File : ClipboardFormat<String>
}

data class ClipboardItem(val elements: List<ClipboardElement<*>>) {
    constructor(vararg elements: ClipboardElement<*>) : this(elements.toList())

    companion object {
        operator fun <T : Any> invoke(value: T, format: ClipboardFormat<T>): ClipboardItem =
            ClipboardItem(listOf(ClipboardElement(value, format)))
    }
}
data class ClipboardElement<T : Any>(
    val value: T,
    val format: ClipboardFormat<T>,
)

interface ClipboardEntryBuilderScope {
    fun add(element: ClipboardElement<*>)
    fun <T : Any> add(value: T, format: ClipboardFormat<T>)
    fun <T : Any> add(values: List<T>, format: ClipboardFormat<T>)
    fun add(elements: List<ClipboardElement<*>>)
}

private class ClipboardEntryBuilderImpl : ClipboardEntryBuilderScope {
    private val sources = mutableListOf<List<ClipboardElement<*>>>()

    override fun add(element: ClipboardElement<*>) {
        sources.add(listOf(element))
    }

    override fun <T : Any> add(value: T, format: ClipboardFormat<T>) {
        sources.add(listOf(ClipboardElement(value, format)))
    }

    override fun <T : Any> add(values: List<T>, format: ClipboardFormat<T>) {
        sources.add(values.map { value -> ClipboardElement(value, format) })
    }

    override fun add(elements: List<ClipboardElement<*>>) {
        sources.add(elements)
    }

    fun build(): List<ClipboardItem> {
        if (sources.isEmpty()) return emptyList()

        val maxSize = sources.maxOf { it.size }
        return (0 until maxSize).map { index ->
            val elements = sources.mapNotNull { source ->
                source.getOrNull(index)
            }
            ClipboardItem(elements)
        }
    }
}
