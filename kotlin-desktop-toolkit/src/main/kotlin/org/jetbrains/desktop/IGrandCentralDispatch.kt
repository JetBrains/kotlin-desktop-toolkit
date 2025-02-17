package org.jetbrains.desktop

public interface IGrandCentralDispatch {
    public fun isMainThread(): Boolean
    public fun dispatchOnMain(f: () -> Unit)
}
