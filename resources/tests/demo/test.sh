#!/usr/bin/env bash
set -euo pipefail

APP_PID=""

cleanup_all() {
  echo "Stopping all dangling engine processes..."
  pkill -9 -f "target/debug/cli engine" || true
}

# whenever this script exits for any reason,
# call the cleanup function first
trap cleanup_all EXIT

cleanup() {
  if [ -n "${APP_PID:-}" ]; then
      if kill -0 "$APP_PID" 2>/dev/null; then
        echo "Stopping engine (pid=$APP_PID)..."
        kill "$APP_PID" || true
        wait "$APP_PID" || true
      fi
  fi
}

healthcheck() {
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "Engine process died early, logs:"
    cat "$LOG_FILE"
    exit 1
  fi
}

# -- PREPARE TEST ---
make demo.setup

# Load environment variables
ENV_FILE="$(dirname "$0")/.dev-env"
if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1091
  source "$ENV_FILE"
else
  echo "Missing .dev-env file!"
  exit 1
fi

LOG_FILE="$(dirname "$0")/demo-engine.log"
rm -f "$LOG_FILE"
touch "$LOG_FILE"

if grep -q '^LOG_FILE=' "$ENV_FILE"; then
  # replace existing line
  sed -i '' -e "s|^LOG_FILE=.*|LOG_FILE=$LOG_FILE|" "$ENV_FILE"
else
  # append if not present
  echo "LOG_FILE=$LOG_FILE" >> "$ENV_FILE"
fi

DB_FILE="$(dirname "$0")/demo-engine.db"
rm -f "$DB_FILE"
touch "$DB_FILE"

if grep -q '^DB_FILE=' "$ENV_FILE"; then
  # replace existing line
  sed -i '' -e "s|^DB_FILE=.*|DB_FILE=$DB_FILE|" "$ENV_FILE"
else
  # append if not present
  echo "DB_FILE=$DB_FILE" >> "$ENV_FILE"
fi

# -- START TEST ---

# simulate historical activity
make demo.play

# start engine on background, pipeing logs
make demo.engine >"$LOG_FILE" 2>&1 &
# give it a moment to spawn
sleep 1
APP_PID=$(pgrep -f "target/debug/cli engine" | head -n1)
echo "Engine pid=$APP_PID"
# wait a bit and check process still alive
sleep 1
healthcheck

# wait for engine to backfill
sleep 1

# simulate live activity
make demo.play

# wait for engine to process
sleep 1

# stop engine
cleanup

# query engine outcomes
make demo.query.checkpoint | jq
make demo.query.transfer | jq
