#!/usr/bin/env bash

# Detect OS to set correct netcat options
# Linux (Arch) uses `nc -q 0` to close connection after EOF
# macOS/BSD uses `nc -c` for the same behavior
NC_OPTS="-q 0" # Default for Linux
if [[ "$(uname -s)" == "Darwin" ]]; then
  NC_OPTS="-c"
fi

printf 'FILE LIST\n' | nc ${NC_OPTS} 127.0.0.1 7000