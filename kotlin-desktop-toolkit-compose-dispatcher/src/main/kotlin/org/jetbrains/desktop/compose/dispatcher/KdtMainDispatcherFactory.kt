package org.jetbrains.desktop.compose.dispatcher

import kotlinx.coroutines.InternalCoroutinesApi
import kotlinx.coroutines.MainCoroutineDispatcher
import kotlinx.coroutines.internal.MainDispatcherFactory
import org.jetbrains.desktop.compose.KdtDispatcher

/**
 * Makes `Dispatchers.Main` the toolkit's event loop thread.
 *
 * kotlinx.coroutines resolves `Dispatchers.Main` once per process, through a [MainDispatcherFactory] found
 * on the classpath with `ServiceLoader` — which is how `Dispatchers.Main` means the Android main looper in
 * one application and the Swing EDT in another. This module is that service for kotlin-desktop-toolkit, so
 * anything written against `Dispatchers.Main` — a library that knows nothing about this toolkit included —
 * lands on the thread the toolkit's windows are driven from.
 *
 * It is a separate artifact from `kotlin-desktop-toolkit-compose` precisely because it takes over a
 * process-wide setting the moment it is on the classpath. The facade itself never needs it: it dispatches
 * through [KdtDispatcher] explicitly, so an application that would rather leave `Dispatchers.Main` alone —
 * or give it to another UI toolkit sharing the process — simply does not depend on this module.
 *
 * `Dispatchers.Main` is therefore usable only while an application is running; before that, dispatching
 * fails the same way [org.jetbrains.desktop.compose.currentApplication] does, because there is no event
 * loop to dispatch to yet.
 */
@OptIn(InternalCoroutinesApi::class)
public class KdtMainDispatcherFactory : MainDispatcherFactory {
    /**
     * Zero, matching the factories kotlinx.coroutines ships for Swing and JavaFX.
     *
     * There is nothing to outrank: a process driven by this toolkit has no second UI thread to lose a tie
     * over, and if one is somehow on the classpath, which of the two `Dispatchers.Main` means is a decision
     * for whoever assembled that classpath rather than for this module to force.
     */
    override val loadPriority: Int
        get() = 0

    override fun createDispatcher(allFactories: List<MainDispatcherFactory>): MainCoroutineDispatcher {
        // Resolves the application per dispatch, so this can be created before one exists — which it is,
        // since `Dispatchers.Main` is initialized on first touch and cached for the life of the process.
        return KdtDispatcher
    }
}
