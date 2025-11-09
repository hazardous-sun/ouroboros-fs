#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Usage Function ---
usage() {
  echo "Usage: $0 -p <port>" >&2
  echo "  -p, --port    The port of the node to kill." >&2
  echo "Example: $0 -p 7001" >&2
  exit 1
}

# --- Defaults ---
PORT=""

# --- Parse Options ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    -p | --port)
      if [[ -z "$2" || "$2" == -* ]]; then echo "Error: $1 requires an argument." >&2; usage; fi
      PORT="$2"
      shift 2
      ;;
    --help)
      usage
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      ;;
  esac
done

# --- Validate ---
if [ -z "${PORT}" ]; then
  echo "Error: -p, --port is a required argument." >&2
  usage
fi

# Check if lsof is installed
if ! command -v lsof &> /dev/null; then
  echo "Error: 'lsof' command not found. Please install it to use this script." >&2
  exit 1
fi

# Find the PID listening on the specified port.
# -iTCP:${PORT} : Find processes with TCP on this port.
# -sTCP:LISTEN : Filter for processes in the LISTEN state.
# -n           : Do not resolve hostnames (faster).
# -P           : Do not resolve port names (faster).
# -t           : Output *only* the Process ID (PID).
PID=$(lsof -iTCP:"${PORT}" -sTCP:LISTEN -n -P -t | head -n 1)

if [ -z "$PID" ]; then
  echo "No process found listening on port ${PORT}."
else
  echo "Found process $PID on port ${PORT}. Sending SIGTERM (kill)..."
  # Kill the process
  kill "$PID"
  echo "Process $PID killed."
fi
