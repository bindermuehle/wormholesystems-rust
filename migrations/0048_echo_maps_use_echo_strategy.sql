-- The maps that named themselves with the echo alias scheme now use the echo strategy.
-- `alias_scheme` goes back to `numeric`, which echo does not read; Postgres cannot drop
-- the `echo` value from that type, so it stays there unused.
update maps
   set naming_strategy = 'echo',
       alias_scheme = 'numeric'
 where alias_scheme = 'echo';
