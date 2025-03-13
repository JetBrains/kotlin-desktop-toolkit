package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeOnInsertText
import org.jetbrains.desktop.macos.generated.NativeOnSetMarkedText
import org.jetbrains.desktop.macos.generated.NativeOnUnmarkText
import org.jetbrains.desktop.macos.generated.NativeSetMarkedTextOperation
import org.jetbrains.desktop.macos.generated.NativeTextInputClient
import org.jetbrains.desktop.macos.generated.NativeTextRange
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
//
//fun is_ime_navigation_key(key_event_info: &KeyEventInfo): Boolean {
//    const ESC_KEYCODE: u32 = 0x1b; // 27
//    let first_char: Option<u32> = if key_event_info.chars.length() > 0 {
//        Some(unsafe { key_event_info.chars.characterAtIndex(0).into() })
//    } else {
//        None
//    };
//    first_char.map_or(true, |ch| {
//        (NSUpArrowFunctionKey..=NSRightArrowFunctionKey).contains(&ch) || ch == ESC_KEYCODE
//    })
//}
//
//fun has_function_modifier(key_event_info: &KeyEventInfo): Boolean {
//    return if (key_event_info.modifiers.contains(NSEventModifierFlags::Function.0)) {
//        val first_char: Option<u32> = if key_event_info.chars.length() > 0 {
//            Some(unsafe { key_event_info.chars.characterAtIndex(0).into() })
//        } else {
//            None
//        };
//        first_char.map_or(true, |ch| !(NSUpArrowFunctionKey..=NSModeSwitchFunctionKey).contains(&ch))
//    } else {
//        false
//    }
//}

public object TextInputContext {
    internal var lastEventHandled = false
    public fun handleCurrentEvent(/*textInputClient: TextInputClient*/): Boolean {
//        if (textInputClient.hasMarkedText()
//            || is_ime_navigation_key(&key_event_info)
//                && !key_event_info.modifiers.contains(NSEventModifierFlags::Control.0)
//                && !has_function_modifier(&key_event_info))
//        ){
//            desktop_macos_h.text_input_context_handle_current_event() //|| self.handle_event(&key_event)
//        } else {
//            self.handle_event(&key_event) || desktop_macos_h.text_input_context_handle_current_event()
//        }
        return desktop_macos_h.text_input_context_handle_current_event()
    }
}

public class SetMarkedTextOperation(
    public val text: String,
    public val selectedRange: TextRange,
    public val replacementRange: TextRange,
)

public interface TextInputClient {
    public fun insertText(text: String)
    public fun doCommand(command: String)
    public fun hasMarkedText(): Boolean
    public fun unmarkText()
    public fun setMarkedText(operation: SetMarkedTextOperation)
}

public class TextRange(
    public val location: Long,
    public val length: Long,
) {
    internal companion object {
        internal fun fromNative(native: MemorySegment): TextRange {
            return TextRange(NativeTextRange.location(native), NativeTextRange.length(native))
        }
    }
}

public data class TextInputClientHolder(var textInputClient: TextInputClient?): AutoCloseable {
    private val arena = Arena.ofShared()

    // called from native code
    private fun onInsertText(text: MemorySegment) {
        ffiUpCall {
            textInputClient?.let {
                it.insertText(text.getUtf8String(0))
                TextInputContext.lastEventHandled = true
            }
        }
    }

    // called from native code
    private fun onDoCommand(command: MemorySegment) {
        ffiUpCall {
            textInputClient?.let {
                it.doCommand(command.getUtf8String(0))
                TextInputContext.lastEventHandled = true
            }
        }
    }

    // called from native code
    private fun onUnmarkText() {
        ffiUpCall {
            textInputClient?.let {
                it.unmarkText()
                TextInputContext.lastEventHandled = true
            }
        }
    }

    private fun onSetMarkedText(s: MemorySegment) {
        val text = NativeSetMarkedTextOperation.text(s).getUtf8String(0)
        val selectedRange = TextRange.fromNative(NativeSetMarkedTextOperation.selected_range(s))
        val replacementRange = TextRange.fromNative(NativeSetMarkedTextOperation.replacement_range(s))
        ffiUpCall {
            textInputClient?.let {
                it.setMarkedText(SetMarkedTextOperation(text, selectedRange = selectedRange, replacementRange = replacementRange))
                TextInputContext.lastEventHandled = true
            }
        }
    }

    internal fun toNative(): MemorySegment {
        val native = NativeTextInputClient.allocate(arena)
        NativeTextInputClient.on_insert_text(native, NativeOnInsertText.allocate(this::onInsertText, arena))
        NativeTextInputClient.on_do_command(native, NativeOnInsertText.allocate(this::onDoCommand, arena))
        NativeTextInputClient.on_unmark_text(native, NativeOnUnmarkText.allocate(this::onUnmarkText, arena))
        NativeTextInputClient.on_set_marked_text(native, NativeOnSetMarkedText.allocate(this::onSetMarkedText, arena))
        return native
    }

    override fun close() {
        arena.close()
    }
}