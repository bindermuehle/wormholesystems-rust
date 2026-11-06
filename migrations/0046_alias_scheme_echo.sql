-- no-transaction
--
-- The ECHO chain naming scheme, `<branch><type><slot>`: a branch letter per direct link
-- off home, the destination's class, then the lowest free slot letter. Carried over from
-- the legacy app, where it is the scheme seven of our maps already use, so their exports
-- do not import without it.
--
-- `alter type ... add value` cannot run inside a transaction and migrations do, hence the
-- directive above. It also cannot be undone: Postgres has no `drop value`.
alter type alias_scheme add value 'echo';
