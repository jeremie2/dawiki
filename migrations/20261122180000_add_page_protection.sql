ALTER TABLE pages ADD COLUMN is_protected INTEGER NOT NULL DEFAULT 0 CHECK (is_protected IN (0, 1));
