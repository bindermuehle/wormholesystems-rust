-- Structure alerts: a corporation's structures under attack or running out of fuel, told
-- to the maps its members chose to tell.
--
-- Everything here is read with one character's token and stays tied to that character. A
-- map sees an event because a member who received it shares with the map, and a fuel
-- timer because a member who can read the list does. A member without the in-game roles
-- shares nothing they could not see themselves.

-- Off unless the user turns it on, per map. A grant on a map is given, not asked for, so
-- without this anyone could add a corporation to their map and read its structures.
alter table map_user_settings add column structures_shared boolean not null default false;
create index map_user_settings_structures on map_user_settings (user_id) where structures_shared;

-- The characters whose structures a map may see: every character of a user who shares
-- with it and still has a grant on it.
create view map_structure_sharers as
    select mus.map_id, c.id as character_id, c.corporation_id
    from map_user_settings mus
    join map_user_roles r on r.map_id = mus.map_id and r.user_id = mus.user_id
    join characters c on c.user_id = mus.user_id
    where mus.structures_shared;

-- Structure fuel only: how many days before the fuel runs out the alert fires.
alter table map_alerts add column fuel_days integer;

-- Structure alerts only: which events the alert speaks about, by `structure_events.kind`.
-- Null is the alert kind's usual set, so the choice only has to be stored once made.
alter table map_alerts add column structure_events text[];

-- One thing that happened to a structure, from an in-game notification. Several characters
-- of a corporation receive the same one, so it is stored once per corporation.
create table structure_events (
    id                       bigint primary key generated always as identity,
    corporation_id           bigint      not null,
    -- `under_attack` | `lost_shields` | `lost_armor` | `reinforced` | `destroyed`
    -- | `fuel_low` | `low_power` | `reagents_low`.
    kind                     text        not null,
    -- EVE's own name for it, e.g. `StructureUnderAttack`, `TowerAlertMsg`.
    notification_type        text        not null,
    -- Whichever of these the notification names. An Upwell structure or a skyhook has an
    -- id of its own; a tower is known by its moon and a customs office by its planet.
    structure_id             bigint,
    moon_id                  bigint,
    planet_id                bigint,
    type_id                  bigint,
    solar_system_id          bigint,
    aggressor_character_id   bigint,
    aggressor_corporation_id bigint,
    aggressor_alliance_id    bigint,
    -- As the notification spelled it, for the ones that give a name and no id.
    aggressor_corporation    text,
    -- Percent left, where the notification says.
    shield                   real,
    armor                    real,
    hull                     real,
    -- When a reinforcement timer it started runs out.
    timer_ends_at            timestamptz,
    occurred_at              timestamptz not null,
    -- Corporation, type, what it was about and when: the same for every recipient.
    dedup_key                text        not null unique,
    created_at               timestamptz not null default now()
);

create index structure_events_age on structure_events (occurred_at);

-- Which characters received an event, which is what decides the maps that see it.
create table structure_event_receipts (
    structure_event_id bigint not null references structure_events (id) on delete cascade,
    character_id       bigint not null references characters (id) on delete cascade,
    primary key (structure_event_id, character_id)
);

create index structure_event_receipts_character on structure_event_receipts (character_id);

-- A corporation's Upwell structures as ESI last listed them, for the fuel timers.
create table corporation_structures (
    structure_id    bigint primary key,
    corporation_id  bigint      not null,
    name            text,
    type_id         bigint,
    solar_system_id bigint,
    state           text        not null,
    fuel_expires    timestamptz,
    state_timer_end timestamptz,
    updated_at      timestamptz not null default now()
);

create index corporation_structures_corporation on corporation_structures (corporation_id);

-- The characters ESI let read their corporation's list (it takes the Station Manager
-- role), and when. One that is refused is removed.
create table corporation_structure_readers (
    character_id   bigint primary key references characters (id) on delete cascade,
    corporation_id bigint      not null,
    read_at        timestamptz not null default now()
);
