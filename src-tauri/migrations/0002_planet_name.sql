-- Add planet_name to notes for display in history view.
-- Existing rows will have NULL; the UI handles that gracefully.
ALTER TABLE notes ADD COLUMN planet_name TEXT;
