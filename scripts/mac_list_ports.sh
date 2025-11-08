#!/usr/bin/env bash

lsof -iTCP -sTCP:LISTEN -n -P | grep rust_sock