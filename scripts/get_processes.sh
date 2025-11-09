#!/usr/bin/env bash

# Lists all running 'ouroboros' processes.
# The [o]uroboros trick prevents grep from matching itself.
echo "Listing running 'ouroboros_fs' processes:"
ps aux | grep "[o]uroboros_fs"
