package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeImage
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout

/**
 * Represents an image that can be used with macOS APIs.
 */
public class Image(private val data: ByteArray) {
    /**
     * Converts this image to a native representation.
     * The returned MemorySegment is allocated in the provided arena.
     */
    internal fun toNative(arena: Arena): MemorySegment {
        val dataSegment = arena.allocateArray(ValueLayout.JAVA_BYTE, *data)
        val imageStruct = NativeImage.allocate(arena)
        NativeImage.data(imageStruct, dataSegment)
        NativeImage.data_length(imageStruct, dataSegment.byteSize())
        return imageStruct
    }

    public companion object {
        /**
         * Creates an Image from a byte array containing image data.
         */
        public fun fromBytes(bytes: ByteArray): Image = Image(bytes)
    }
}
