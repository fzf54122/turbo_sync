ALTER TABLE nodes ADD COLUMN health_status TEXT NOT NULL DEFAULT 'unchecked';
ALTER TABLE nodes ADD COLUMN health_message TEXT;
ALTER TABLE nodes ADD COLUMN last_checked_at TEXT;
