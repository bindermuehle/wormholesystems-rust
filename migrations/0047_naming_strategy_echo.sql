-- no-transaction
--
-- The ECHO scheme becomes a naming strategy of its own, next to `chain` and `siblings`.
-- It was an alias scheme while naming happened in the browser; now the server names
-- things, and echo decides more than the sequence a chain walks (which branch, which slot,
-- which hole is home's static), which is what a strategy is.
--
-- `alter type ... add value` cannot run inside a transaction, hence the directive above,
-- and the new value cannot be used in the migration that adds it, hence 0048.
alter type naming_strategy add value 'echo';
