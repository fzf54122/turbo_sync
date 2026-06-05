ALTER TABLE sync_tasks ADD COLUMN conflict_mode TEXT NOT NULL DEFAULT 'newest_wins';
