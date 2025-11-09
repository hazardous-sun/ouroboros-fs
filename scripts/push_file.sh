#!/usr/bin/env bash

# Detect OS to set correct netcat options
# Linux (Arch) uses `nc -q 0`
# macOS/BSD uses `nc -w 1`
NC_OPTS="-q 0" # Default for Linux

# Detect OS to set correct wc (word count) options
# Linux (Arch) uses GNU `wc --bytes`
# macOS/BSD uses `wc -c`
WC_OPTS="--bytes" # Default for Linux

if [[ "$(uname -s)" == "Darwin" ]]; then
  NC_OPTS="-w 1"
  WC_OPTS="-c"
fi

# --- Defaults ---
HOST="127.0.0.1"
PORT="7000"
LOCAL_FILE=""

# --- Usage Function ---
usage() {
  echo "Usage: $0 -f <local_file_path> [-h <host>] [-p <port>]" >&2
  echo "  -f, --file    Path to the local file to push." >&2
  echo "  -h, --host    Network host (default: 127.0.0.1)." >&2
  echo "  -p, --port    Network port (default: 7000)." >&2
  echo "Example: $0 -f ./Cargo.toml" >&2
  exit 1
}

# --- Parse Options ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    -f | --file)
      if [[ -z "$2" || "$2" == -* ]]; then echo "Error: $1 requires an argument." >&2; usage; fi
      LOCAL_FILE="$2"
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
if [ -z "${LOCAL_FILE}" ]; then
  echo "Error: -f, --file is a required argument." >&2
  usage
fi

if [ ! -e "${LOCAL_FILE}" ]; then
  echo "Error: File not found: ${LOCAL_FILE}" >&2
  exit 1
fi

# --- Action ---
# Get the byte count from wc and pipe to `xargs` to trim whitespace (for BSD wc)
SIZE_STR=$(wc ${WC_OPTS} < "${LOCAL_FILE}" | xargs)
# Get just the filename from the path
FILE_NAME=$(basename "${LOCAL_FILE}")

echo "Pushing '${LOCAL_FILE}' as '${FILE_NAME}' (${SIZE_STR} bytes) to ${HOST}:${PORT}..."
# Build message header and body, then send it to a node using netcat
( printf "FILE PUSH ${SIZE_STR} ${FILE_NAME}\n"; cat "${LOCAL_FILE}" ) | nc ${NC_OPTS} ${HOST} ${PORT}
