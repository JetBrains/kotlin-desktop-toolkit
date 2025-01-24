#!/usr/bin/env bash

cd "$(dirname "$0")"
cd .. # project root

cd native
cargo build
cd ..

rm scripts/filtered_headers/headers.txt

jextract \
--include-dir /Library/Developer/CommandLineTools/usr/lib/clang/16.0.0/include \
--include-dir /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include \
--dump-includes scripts/filtered_headers/headers.txt \
native/kwm-macos/headers/kwm_macos.h 2> /dev/null

rm scripts/filtered_headers/filtered_headers.txt
grep "wm_macos.h$" scripts/filtered_headers/headers.txt > scripts/filtered_headers/filtered_headers.txt

rm -rf lib/src/main/java/org/jetbrains/kwm/macos/generated/*

jextract \
  --include-dir /Library/Developer/CommandLineTools/usr/lib/clang/16.0.0/include \
  --include-dir /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include \
  --output lib/src/main/java \
  --target-package org.jetbrains.kwm.macos.generated \
  --library kwm_macos \
  @scripts/filtered_headers/filtered_headers.txt \
  native/kwm-macos/headers/kwm_macos.h 2> /dev/null
