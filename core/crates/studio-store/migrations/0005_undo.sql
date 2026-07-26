-- What a change replaced, so it can be undone.
--
-- The change log said what happened and who did it, which is enough to show a history and not
-- enough to reverse one. Undo is the first thing anyone reaches for in a spreadsheet, and
-- offering it without this would mean guessing at what was there before.
--
-- Stored as the cells and their previous values, in the file's own terms, because that is what
-- putting it back requires. Null for changes made before this existed, and for kinds of change
-- that carry no cell values — a history entry that cannot be undone says so rather than
-- offering a button that fails.
ALTER TABLE artefact_changes ADD COLUMN undo_cells TEXT;

-- Which sheet the cells belong to. Without it, "B6" is ambiguous in a workbook with four sheets.
ALTER TABLE artefact_changes ADD COLUMN undo_where TEXT;
