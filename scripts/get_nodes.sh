#!/usr/bin/env bash

# Detect OS to set correct netcat options
# Linux (Arch) uses `nc -q 0` to close connection after EOF
# macOS/BSD uses `nc -w 1` (1-second timeout)
NC_OPTS="-q 0" # Default for Linux
if [[ "$(uname -s)" == "Darwin" ]]; then
  NC_OPTS="-w 1"
fi

# --- Defaults ---
HOST="127.0.0.1"
PORT="7000"

# --- Usage Function ---
usage() {
  echo "Usage: $0 [-h <host>] [-p <port>]" >&2
  echo "  -h, --host    Network host (default: 127.0.0.1)." >&2
  echo "  -p, --port    Network port (default: 7000)." >&2
  exit 1
}

# --- Parse Options ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --host)
      if [[ -z "$2" || "$2" == -* ]]; then echo "Error: $1 requires an argument." >&2; usage; fi
      HOST="$2"
      shift 2
      ;;
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

printf 'NETMAP GET\n' | nc ${NC_OPTS} ${HOST} ${PORT}
