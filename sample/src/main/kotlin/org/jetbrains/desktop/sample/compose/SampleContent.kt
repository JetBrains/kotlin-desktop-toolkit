package org.jetbrains.desktop.sample.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * The Compose content that will be hosted in a kotlin-desktop-toolkit window.
 *
 * Nothing renders this yet — [ComposeWindow] still draws a placeholder, and hooking the two together
 * needs a `ComposeScene` driven from the window's draw and event callbacks. For now this exists so the
 * Compose dependencies and the Compose compiler plugin are exercised by the build.
 */
@Composable
fun SampleContent() {
    MaterialTheme(colorScheme = darkColorScheme()) {
        var clicks by remember { mutableStateOf(0) }
        Column(
            modifier = Modifier.fillMaxSize().padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = "Compose in a kotlin-desktop-toolkit window",
                style = MaterialTheme.typography.titleMedium,
            )
            Button(onClick = { clicks++ }) {
                Text("Clicked $clicks times")
            }
        }
    }
}
