#!/usr/bin/env bash

printf 'NETMAP GET\n' | nc -q 0 127.0.0.1 7000
