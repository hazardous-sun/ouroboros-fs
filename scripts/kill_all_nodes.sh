#!/usr/bin/env bash

# Find and kill all processes containing "ouroboros"
#
# - ps aux: List all processes
# - grep ouroboros: Find processes matching the name
# - grep -v grep: Remove the grep command itself from the list
# - awk '{print $2}': Get only the second column (the PID)
# - xargs kill -9: Send SIGKILL to each PID
#
# Using kill -9 (SIGKILL) is a forceful termination.
# Use 'kill -15' (SIGTERM) for a more graceful shutdown.

PIDS=$(ps aux | grep "[o]uroboros" | awk '{print $2}')

if [ -z "$PIDS" ]; then
  echo "No processes found containing 'ouroboros'."
else
  echo "Killing 'ouroboros' processes with PIDs:"
  echo "$PIDS"
  # The [o]uroboros trick in grep prevents grep from matching itself,
  # but we use xargs which handles empty input gracefully.
  # Using kill -9 for a forceful stop.
  echo "$PIDS" | xargs kill -9
  echo "Done."
fi
