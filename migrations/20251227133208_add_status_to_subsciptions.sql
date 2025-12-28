-- Migration to add 'status' column to 'subscriptions' table
ALTER TABLE subscriptions ADD COLUMN status TEXT NULL;