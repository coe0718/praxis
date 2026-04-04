#!/usr/bin/env bash
set -euo pipefail

data_dir="${1:?usage: check-heartbeat.sh <data-dir> [max-age-seconds]}"
max_age="${2:-900}"

exec praxis --data-dir "$data_dir" heartbeat check --max-age-seconds "$max_age"
