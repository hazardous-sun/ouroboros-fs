#!/usr/bin/env bash

# Detect OS to set correct netcat options
# Linux (Arch) uses `nc -q 0`
# macOS/BSD uses `nc -w 1`
NC_OPTS="-q 0" # Default for Linux
if [[ "$(uname -s)" == "Darwin" ]]; then
  NC_OPTS="-w 1"
fi

# --- Defaults ---
HOST="127.0.0.1"
PORT="7000"
FILE_NAME=""

# --- Usage Function ---
usage() {
  echo "Usage: $0 -f <file_name_on_server> [-h <host>] [-p <port>]" >&2
  echo "Outputs the file to stdout. Redirect to save it." >&2
  echo "  -f, --file    Name of the file to pull from the network." >&2
  echo "  -h, --host    Network host (default: 127.0.0.1)." >&2
  echo "  -p, --port    Network port (default: 7000)." >&2
  echo "Example: $0 -f Cargo.toml > ./downloaded_cargo.toml" >&2
  exit 1
}

# --- Parse Options ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    -f | --file)
      if [[ -z "$2" || "$2" == -* ]]; then echo "Error: $1 requires an argument." >&2; usage; fi
      FILE_NAME="$2"
      shift 2
      ;;
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

# --- Validate ---
if [ -z "${FILE_NAME}" ]; then
  echo "Error: -f, --file is a required argument." >&2
  usage
fi

# Send the FILE PULL command.
printf "FILE PULL ${FILE_NAME}\n" | nc ${NC_OPTS} ${HOST} ${PORT}
