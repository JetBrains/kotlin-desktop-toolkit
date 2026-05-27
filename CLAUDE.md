# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

[AGENTS.md](AGENTS.md) is the canonical agent guide — build/test/lint commands, the Kotlin↔Rust split, the Win32 doc-set pointer, and platform scope live there. This file only adds Claude-Code-specific notes.

- **[.git-hooks/pre-push](.git-hooks/pre-push) runs `./gradlew lint`** — treat that as the verification bar before claiming a change is done.
