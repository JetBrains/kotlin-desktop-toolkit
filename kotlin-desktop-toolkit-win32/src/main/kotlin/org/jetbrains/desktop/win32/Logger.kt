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

internal inline fun ffiUpCall(crossinline body: () -> Unit) {
    return try {
        body()
    } catch (e: Throwable) {
        //Logger.error(e) { "Exception caught" }
    }
}

internal inline fun <T> ffiUpCall(defaultResult: T, crossinline body: () -> T): T {
    return try {
        body()
    } catch (e: Throwable) {
        //Logger.error(e) { "Exception caught" }
        defaultResult
    }
}
