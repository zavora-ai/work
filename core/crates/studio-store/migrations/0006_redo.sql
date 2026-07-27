-- What an undo replaced, so it can be put forward again.
--
-- Kept apart from `undo_cells` on purpose. If undoing simply recorded itself as an undoable
-- change, pressing undo twice would put the first change back rather than stepping further back —
-- a toggle where every spreadsheet has a stack. So an undo carries what it displaced in its own
-- column, redo consumes that, and undo goes on walking backwards.
ALTER TABLE artefact_changes ADD COLUMN redo_cells TEXT;
