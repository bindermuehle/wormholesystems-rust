---
title: Linking wormholes
---

# Linking wormholes

When a signature is a wormhole, it can be **linked** to the connection it represents on the map. This ties the in-game signature ID to that connection, so the whole group knows exactly which hole in space leads where.

## How a signature gets linked

Linking is **deliberate, not automatic** — pasting a scan won't guess which hole goes where. A signature is tied to a connection when you record the jump through it: as you map [as you fly](/documentation/getting-started/auto-mapping), you pick the signature you jumped through, and that signature is attached to the new connection.

If a signature stops being a wormhole (for example a re-paste reclassifies it), its connection link is cleared automatically.

A connection takes **one signature per side**: the signature is the hole, so two of them on the same side would be two scans of one hole. Two holes in a system can lead to the same place, though, and then they are two connections, each with its own signature, mass and lifetime. So picking a system that another signature already leads to, listed under **Another hole to**, draws a second connection beside the first and leaves the first signature where it was. Resolving an [unmapped hole](/documentation/signatures/unmapped-holes) into a system another signature already leads to does the same. To move a connection from one signature to another instead, set the first back to **Unknown** and then pick it for the second.

## Why it's worth doing

- The connection shows its real signature ID, matching your scanner.
- When a system has several wormholes, each one maps to a known link — no ambiguity.
- A K162 typed as a range (C1-3 or C4-5) becomes the exact class once its connection leads to a system on the map, and the type list only offers what can lead there.
- A linked wormhole carries the connection's [lifetime](/documentation/connections/lifetime), [mass](/documentation/connections/mass), and [ship-size](/documentation/connections/ship-size) status, all describing the same physical hole.
