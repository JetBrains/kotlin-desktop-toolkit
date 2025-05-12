[![internal JetBrains project](https://jb.gg/badges/internal.svg)](https://confluence.jetbrains.com/display/ALL/JetBrains+on+GitHub)
# Kotlin Window Management library


## Goals
* Provide a simple enough Kotlin API for interacting with OS
* Support all major desktop platforms: Linux, MacOS and Windows for now
* Make it Kotlin Multiplatform in future

## Non-Goals
* Providing bindings for Android or iOS. It's too different from Desktop
* Bindings for browser API
* API alignment across platforms. Desktop platforms are different and provide different capabilities, we are not going to hide this difference. Though for some common APIs we could provide a layer that simplifies usage of it across the platforms
