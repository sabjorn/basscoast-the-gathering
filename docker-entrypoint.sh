#!/bin/bash
set -e

DB_PATH="/app/data/basscoast.db"

# Only import if database doesn't exist
# If it exists (even if empty), assume it's intentional user data
if [ ! -f "$DB_PATH" ]; then
    echo "Database not found. Running initial data import..."
    # SQLx requires the file to exist before it can connect
    touch "$DB_PATH"
    import-json /app/data/bass_coast_artists_history.json
    echo "Data import complete."
else
    echo "Database exists. Skipping import."
fi

# Execute the command passed to docker run (defaults to CMD in Dockerfile)
echo "Starting Bass Coast: The Gathering..."
exec "$@"
