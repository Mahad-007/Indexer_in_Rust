#!/bin/bash
set -e

# Function to cleanup background processes on exit
cleanup() {
    echo "Stopping background processes..."
    # Kill all child processes of this script
    pkill -P $$
    exit
}

trap cleanup SIGINT SIGTERM

echo "============================================"
echo "   Starting Indexer Service (Dev Mode)      "
echo "============================================"

# 1. Load Environment Variables
if [ -f .env ]; then
    echo "Loading .env file..."
    set -a
    source .env
    set +a
else
    echo "Error: .env file not found!"
    echo "Please copy .env.example to .env and configure it."
    exit 1
fi

# Override for local execution
# We need to use localhost because we are running outside docker
export PGHOST=localhost
export REDIS_URL=redis://localhost:6379
# Ensure DATABASE_URL uses localhost
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@localhost:${PGPORT}/${PGDATABASE}"

# 2. Start Infrastructure (DB & Redis)
echo "Starting database and redis..."
docker compose up -d db redis

# Wait for DB to be ready
echo "Waiting for database to be ready..."
until docker compose exec db pg_isready -U ${PGUSER:-beanbee} -d ${PGDATABASE:-beanbee_development}; do
  echo "Database is unavailable - sleeping"
  sleep 2
done
echo "Database is ready!"

# 3. Run Migrations
echo "Running migrations..."
# Check if sqlx-cli is installed
if ! cargo sqlx --version &> /dev/null; then
    echo "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

echo "Using DATABASE_URL: $DATABASE_URL"

pushd libs/indexer-db > /dev/null
cargo sqlx migrate run
popd > /dev/null

# 4. Start Services
echo "Starting Services..."

# Run listener
echo "Starting Listener..."
cargo run -p listener &

# Run processor
echo "Starting Processor..."
cargo run -p processor &

# Run API
echo "Starting API..."
cargo run -p api &

echo "All services started. Press Ctrl+C to stop."
wait
