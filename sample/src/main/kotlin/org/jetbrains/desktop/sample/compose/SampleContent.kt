package org.jetbrains.desktop.sample.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.unit.dp

/**
 * Content hosted in a kotlin-desktop-toolkit window.
 *
 * The button exercises pointer events, the text field exercises key events, and the reported size shows
 * that resizes reach the composition.
 */
@Composable
fun ComposeWindowScope.SampleContent() {
    MaterialTheme(colorScheme = darkColorScheme()) {
        var clicks by remember { mutableStateOf(0) }
        var text by remember { mutableStateOf("") }
        var lastKey by remember { mutableStateOf("none") }
        // Surface paints the theme's background and sets the matching content colour for everything
        // inside it; without one, text falls back to black on whatever the frame was cleared to.
        Surface(modifier = Modifier.fillMaxSize()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .onPreviewKeyEvent { event ->
                        if (event.type == KeyEventType.KeyDown) {
                            lastKey = "${event.key} (codePoint=${event.utf16CodePoint})"
                        }
                        false
                    }
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    text = "Compose in a kotlin-desktop-toolkit window",
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text = "Content size: ${window.contentSize.width.value.toInt()} x ${window.contentSize.height.value.toInt()} dp",
                    style = MaterialTheme.typography.bodySmall,
                )
                Button(onClick = { clicks++ }) {
                    Text("Clicked $clicks times")
                }
                TextField(
                    value = text,
                    onValueChange = { text = it },
                    label = { Text("Type here") },
                )
                Text(
                    text = "Last key down: $lastKey",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}
