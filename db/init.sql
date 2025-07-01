-- Create the database if it doesn't exist
SELECT 'CREATE DATABASE transaction_queue'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'transaction_queue')\gexec

-- Connect to the database
\c transaction_queue;

-- Create UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";