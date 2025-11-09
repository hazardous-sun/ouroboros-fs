#!/usr/bin/env bash

# Detect OS to set correct netcat options
# Linux (Arch) uses `nc -q 0` to close connection after EOF
# macOS/BSD uses `nc -w 1` (1-second timeout)
NC_OPTS="-q 0" # Default for Linux
if [[ "$(uname -s)" == "Darwin" ]]; then
  NC_OPTS="-w 1"
fi

HOST="127.0.0.1"
PORT="${1:-7000}" # Default to port 7000 if no arg provided

echo "Sending NODE HEAL to ${HOST}:${PORT}..."
echo "Waiting for response (this may take a minute)..."

# Send the NODE HEAL command
# This will wait for the server to send a response
printf 'NODE HEAL\n' | nc ${NC_OPTS} ${HOST} ${PORT}