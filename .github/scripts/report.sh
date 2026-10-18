#!/usr/bin/env bash
# Runs a command and, when it fails, turns its errors into one annotation readable from the API.
set -o pipefail
log=$(mktemp)
"$@" 2>&1 | tee "$log"
status=$?
if [ $status -ne 0 ]; then
	msg=$(sed "s/\x1b\[[0-9;]*m//g" "$log" | awk '/panicked/{n=4} n>0{print; n--; next} /error|warning:|FAILED|^ *--> |assertion/' | head -80 | sed 's/%/%25/g' | awk '{printf "%s%%0A", $0}')
	echo "::error title=$1 failed::$msg"
fi
exit $status
