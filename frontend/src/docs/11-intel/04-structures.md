---
title: Structures
---

# Structures

The **Structures** card shows what members share about their corporations' structures:
who is shooting what, and what runs out of fuel next. It is the same information EVE sends
to a director in game, put where the whole group can see it.

Each row names the structure, its system with the map's alias if it has one, the region
and the jumps from your route origin, and answers a right-click with the system's menu like
every other list. The header has a search field (structure, system, alias, region,
corporation or attacker) and three switches to hide attacks, fuel or routine changes.

## Sharing is yours to turn on

Nothing is shared by default. Each member decides, per map, with **Share my structures with
this map** (the button in the card's header, or Settings, Mapping). Turn it off and the map
forgets what it knew through you.

This matters because access to a map is given, not asked for: anyone can add your
corporation to their map. Until you turn sharing on for that map, it sees nothing of yours.

Only members and above see the card's contents. Viewers and share links do not.

## What is read

Sharing reads with each of your characters that granted one of two optional permissions,
from [the characters settings page](/documentation/getting-started/connecting-your-character):

| Permission                 | What the map gets                                                              |
| -------------------------- | ------------------------------------------------------------------------------ |
| **Notifications**          | Attacks, reinforcements and losses, and EVE's own low-fuel warnings.           |
| **Corporation structures** | The fuel timer of every Upwell structure, so the map can warn days in advance. |

A character shares only what it can see in game. Notifications about structures go to
characters with the corporation roles that receive them, and the structure list needs the
**Station Manager** role. A member with neither shares nothing, even with the switch on.

Only notifications about structures are kept. Everything else a character receives is
ignored.

## What it covers

| Structure                   | Attacks | Fuel                                    |
| --------------------------- | ------- | --------------------------------------- |
| Upwell structures           | yes     | timer, plus EVE's warning and low power |
| Control towers (POS)        | yes     | EVE's own warning when the bay runs low |
| Customs offices             | yes     | nothing to fuel                         |
| Skyhooks and mercenary dens | yes     | nothing to fuel                         |
| Metenox moon drills         | yes     | EVE's low-reagent warnings              |

EVE caches a character's notifications for ten minutes, so an attack shows up within about
that long. Several sharing members in one corporation shorten the wait: they are read in
turn, so three of them see it within about three minutes. Fuel timers are read hourly.

## Alerts

An attack that just happened shows as a notice on the map for every member who has it
open. To hear about it in Discord, a manager adds one of the two structure
[alerts](/documentation/intel/alerts): **Structure attacks and changes**, or **Structure
fuel** with the number of days of warning you want. Each lets you tick which events it
speaks about.
