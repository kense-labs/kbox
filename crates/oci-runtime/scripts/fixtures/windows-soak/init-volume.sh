#!/bin/sh
set -eu

log='/mnt/rw/lifecycle.log'
marker='/.koci-create-start-smoke'

on_term() {
    printf 'phase=term\n' >> "$log"
    exit 0
}
trap on_term TERM

test "${K_INIT_SCENARIO:-}" = 'volume-init'
test "${K_INIT_DELAY_SECONDS:-}" = '1'
test "$(pwd)" = '/'

{
    printf 'phase=begin\n'
    printf 'scenario=%s\n' "$K_INIT_SCENARIO"
    printf 'cwd=%s\n' "$(pwd)"
    printf 'state_volume=rw\n'
} > "$log"
sleep "$K_INIT_DELAY_SECONDS"
printf 'phase=ready\n' >> "$log"
printf 'koci-create-start-user-time-v1\n' > "$marker"

while :; do
    sleep 1
done
