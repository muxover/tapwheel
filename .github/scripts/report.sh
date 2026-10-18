#!/usr/bin/env bash
# Runs a command and, when it fails, turns its errors into one annotation readable from the API.
set -o pipefail
log=$(mktemp)
"$@" 2>&1 | tee "$log"
status=$?
if [ $status -ne 0 ]; then
	msg=$(sed "s/\x1b\[[0-9;]*m//g" "$log" | grep -E "error|warning:|panicked|FAILED|^ *--> |assertion" | head -60 | sed 's/%/%25/g' | awk '{printf "%s%%0A", $0}')
	echo "::error title=$1 failed::$msg"
fi
exit $status
