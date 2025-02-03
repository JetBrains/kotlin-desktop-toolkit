package org.jetbrains.kwm

interface IGrandCentralDispatch {
    fun isMainThread(): Boolean
    fun dispatchOnMain(f: () -> Unit)
}