#!/bin/bash
set -e

# Create Hydra database and user
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create hydra user and database
    CREATE USER hydra WITH PASSWORD 'hydra';
    CREATE DATABASE hydra OWNER hydra;
    GRANT ALL PRIVILEGES ON DATABASE hydra TO hydra;

    -- Create application user and database
    CREATE USER app WITH PASSWORD 'app';
    CREATE DATABASE saas_app OWNER app;
    GRANT ALL PRIVILEGES ON DATABASE saas_app TO app;
EOSQL

# Grant schema creation permission for app user (needed for multi-tenant schemas)
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "saas_app" <<-EOSQL
    -- Allow app user to create schemas
    GRANT CREATE ON DATABASE saas_app TO app;
    GRANT ALL ON SCHEMA public TO app;
EOSQL

echo "Databases initialized successfully!"
