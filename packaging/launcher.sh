#!/bin/bash
# Launches your Rust binary inside Terminal.app
exec /usr/bin/osascript -e 'tell application "Terminal"
    do script "'"$(dirname "$0")/fintool"'"
    activate
end tell'