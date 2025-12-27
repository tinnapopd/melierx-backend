BEGIN;
    -- Backfill `status` for historical records
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    
    -- Alter the `status` column to be NOT NULL
    ALTER TABLE subscriptions
        ALTER COLUMN status SET NOT NULL;
COMMIT;