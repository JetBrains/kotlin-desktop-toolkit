#!/usr/bin/env bash

# Colours
green='\033[0;32m'
yellow='\033[0;33m'
reset='\033[0m'

# Check for dry-run mode
DRY_RUN=false
if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${yellow}Running in dry-run mode - no changes will be made${reset}"
fi

# Kill Preferences, just incase it's running, supress potential warning
# about no running process
killall 'System Preferences' &>/dev/null

# Format: "Layout Name:ID"
keyboard_layouts=(
    'USInternational-PC:1500'
    'Swedish - Pro:7'
    'ABC:252'
    'Russian:19456'
    'German:3'
    'Serbian-Latin:-19521'
    'Serbian:19521'
    'Dvorak:16300'
    'DVORAK - QWERTY CMD:16301'
)

# The keys we have to add for each layout
apple_keys=("AppleEnabledInputSources")

# Function to check if input source already exists
input_source_exists() {
    local key="$1"
    local layout_id="$2"
    local layout_name="$3"

    # Get current array contents
    local current=$(defaults -host "${USER}" read com.apple.HIToolbox "$key" 2>/dev/null)

    # Check if the specific layout already exists (format: "KeyboardLayout ID" = 1500;)
    if [[ "$current" == *"\"KeyboardLayout ID\" = ${layout_id};"* ]]; then
        return 0  # exists
    else
        return 1  # doesn't exist
    fi
}

for entry in "${keyboard_layouts[@]}"; do
        layout_name="${entry%:*}"
        layout_id="${entry##*:}"

        for key in "${apple_keys[@]}"; do
                # Only add if it doesn't already exist
                if ! input_source_exists "$key" "$layout_id" "$layout_name"; then
                    echo "Adding $layout_name to $key"
                    if [[ "$DRY_RUN" == "false" ]]; then
                        defaults -host "${USER}" write com.apple.HIToolbox \
                                        "$key" \
                                        -array-add "<dict><key>InputSourceKind</key><string>Keyboard Layout</string>"\
"<key>KeyboardLayout ID</key><integer>${layout_id}</integer>"\
"<key>KeyboardLayout Name</key><string>${layout_name}</string></dict>"
                    else
                        echo "  [DRY-RUN] Would add entry"
                    fi
                else
                    echo "$layout_name already exists in $key, skipping"
                fi
        done
done

# =============================================================================
# Input Methods (e.g., Japanese, Chinese)
# =============================================================================

# Format: "Bundle ID:Input Mode"
input_methods=(
    'com.apple.inputmethod.Kotoeri.RomajiTyping:com.apple.inputmethod.Japanese'
    'com.apple.inputmethod.Kotoeri.KanaTyping:com.apple.inputmethod.Japanese'
    'com.apple.inputmethod.TCIM:com.apple.inputmethod.TCIM.Pinyin'
    'com.apple.inputmethod.Korean:com.apple.inputmethod.Korean.2SetKorean'
)

# Function to check if input method already exists
input_method_exists() {
    local key="$1"
    local bundle_id="$2"

    local current=$(defaults -host "${USER}" read com.apple.HIToolbox "$key" 2>/dev/null)

    if [[ "$current" == *"\"Bundle ID\" = \"${bundle_id}\";"* ]]; then
        return 0  # exists
    else
        return 1  # doesn't exist
    fi
}

for entry in "${input_methods[@]}"; do
    bundle_id="${entry%:*}"
    input_mode="${entry##*:}"

    for key in "${apple_keys[@]}"; do
        if ! input_method_exists "$key" "$bundle_id"; then
            echo "Adding input method $bundle_id to $key"
            if [[ "$DRY_RUN" == "false" ]]; then
                # Add the "Keyboard Input Method" entry
                defaults -host "${USER}" write com.apple.HIToolbox \
                    "$key" \
                    -array-add "<dict><key>Bundle ID</key><string>${bundle_id}</string>"\
"<key>InputSourceKind</key><string>Keyboard Input Method</string></dict>"
                # Add the "Input Mode" entry
                defaults -host "${USER}" write com.apple.HIToolbox \
                    "$key" \
                    -array-add "<dict><key>Bundle ID</key><string>${bundle_id}</string>"\
"<key>Input Mode</key><string>${input_mode}</string>"\
"<key>InputSourceKind</key><string>Input Mode</string></dict>"
            else
                echo "  [DRY-RUN] Would add entry"
            fi
        else
            echo "Input method $bundle_id already exists in $key, skipping"
        fi
    done
done

killall TextInputMenuAgent
killall cfprefsd
sleep 3

echo -e "${green}Successfully set default values for input sources! ${reset}"
