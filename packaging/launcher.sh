#!/bin/bash
# Launches your Rust binary inside Terminal.app
exec /usr/bin/osascript -e 'tell application "Terminal"
    set win to do script "'"$(dirname "$0")/fintool ; exit"'"
    activate
    -- wait until the command finishes
    repeat
        delay 0.5
        if not busy of win then
            close window 1
            exit repeat
        end if
    end repeat
end tell'