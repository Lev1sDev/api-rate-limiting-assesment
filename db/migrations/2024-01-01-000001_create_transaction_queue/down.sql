-- Drop triggers
DROP TRIGGER IF EXISTS update_transaction_queue_updated_at ON transaction_queue;
DROP TRIGGER IF EXISTS update_rate_limits_updated_at ON rate_limits;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop tables
DROP TABLE IF EXISTS rate_limits;
DROP TABLE IF EXISTS transaction_queue;