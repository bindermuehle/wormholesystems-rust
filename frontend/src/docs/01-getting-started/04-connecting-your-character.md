---
title: Connecting your character
---

# Connecting your character

The app talks to EVE Online through **ESI**, CCP's official API. To unlock live features you grant a set of _scopes_ — permissions that let the app read certain data or perform certain actions on your behalf. Manage them any time from the **Scopes** link in the header (or grant them from within a map when it needs them).

## The scopes the app uses

| Scope                            | What it enables                                         |
| -------------------------------- | ------------------------------------------------------- |
| `esi-location.read_location.v1`  | Your current solar system — the basis of live tracking. |
| `esi-location.read_ship_type.v1` | Your current ship's type and name.                      |
| `esi-location.read_online.v1`    | Your online status.                                     |
| `esi-ui.write_waypoint.v1`       | Setting in-game autopilot waypoints from the map.       |
| `publicData`                     | Basic public character information.                     |

The first three power [character tracking](/documentation/tracking/character-presence); the waypoint scope powers [setting waypoints](/documentation/autopilot-and-routing/setting-waypoints). If a scope is missing, the header shows how many are outstanding so you can grant them in one place.

## Optional scopes

| Scope                                  | What it enables                                                                                                      |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `esi-universe.read_structures.v1`      | Names the structures you dock in, for the map.                                                                       |
| `esi-characters.read_notifications.v1` | Attacks on your corporation's structures, for the maps you [share structures](/documentation/intel/structures) with. |
| `esi-corporations.read_structures.v1`  | Your corporation's structure fuel timers, for the same maps. Needs the Station Manager role in game.                 |

Not asked for when you sign in, and never flagged as missing. Grant them from the characters settings page if you want them. The two structure permissions read nothing until you turn sharing on for a map. Public structures are named anyway; a private one is visible only to pilots on its docking list, so the pilots list shows it as an unknown structure until someone with this scope docks in it.

## Multiple characters

You can connect several characters and switch your **active character** at any time. Tracking and waypoint actions apply to the character you choose, and [bulk waypoints](/documentation/autopilot-and-routing/setting-waypoints) can target several of your characters at once.

> Scopes are under your control. Revoke them at any time from the Scopes page (which removes the app's tokens for that character) or from EVE's third-party application settings.

When EVE stops accepting a character's token (access revoked in EVE's settings, for example), the app stops tracking that character and the characters page says **EVE signed this character out**. Log in with it again to pick up where it left off.
