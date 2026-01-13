#!/usr/bin/env bash

# Colours
red='\033[0;31m'
green='\033[0;32m'
reset='\033[0m'

# Kill Preferences, just incase it's running, supress potential warning
# about no running process
killall 'System Preferences' &>/dev/null

layouts=('USInternational-PC' 'Swedish - Pro' 'ABC')
ids=('1500' '7' '252')

if [ ! ${#layouts[@]} -eq ${#ids[@]} ] ; then
        echo -e "${red}Number of layout names does not equal number of ids! ${reset}"
        # Like pressing CTRL+C
        kill -INT $$
fi

# The keys we have to add for each layout
apple_keys=("AppleEnabledInputSources" "AppleSelectedInputSources")

# Create the XML entries with defaults write
defaults -host "${USER}" write com.apple.HIToolbox AppleCurrentKeyboardLayoutInputSourceID com.apple.keylayout.Swedish-Pro

for ((i=0 ; i<${#layouts[@]}; i++)) ; do
        for key in ${apple_keys[@]} ; do
                defaults -host "${USER}" write com.apple.HIToolbox \
                                $key \
                                -array-add "<dict><key>InputSourceKind</key><string>Keyboard Layout</string>"\
"<key>KeyboardLayout ID</key><integer>${ids[i]}</integer>"\
"<key>KeyboardLayout Name</key><string>${layouts[i]}</string></dict>"
        done
done

killall TextInputMenuAgent
killall cfprefsd
sleep 3

echo -e "${green}Successfully set default values for input sources and the dock! ${reset}"