-- The route drawn next to a jump count in the lists: a mark per system on the way, in its
-- class colour. On unless turned off; some find a list of them too busy.
alter table user_preferences
    add column route_previews boolean not null default true;
