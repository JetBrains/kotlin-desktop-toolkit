package org.jetbrains.desktop

interface IGrandCentralDispatch {
    fun isMainThread(): Boolean
    fun dispatchOnMain(f: () -> Unit)
}