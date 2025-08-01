package org.jetbrains.desktop.win32

internal fun <T> ffiDownCall(body: () -> T): T {
    val result = body()
//    val exceptions = checkExceptions()
//    if (exceptions.isNotEmpty()) {
//        desktop_windows_h.logger_clear_exceptions()
//        throw NativeError(exceptions)
//    }
    return result
}
