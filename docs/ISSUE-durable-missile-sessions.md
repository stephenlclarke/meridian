# Durable missile sessions

## Problem and motivation

The delivered finite launcher refuses saving an armed, locked or used session because version 7 cannot retain consumed items and complete ordnance history. This prevents Save/Quit/Continue after a normal missile launch.

## Required behavior

Version 8 must retain actual owned/consumed pylon identities, complete live and ended projectiles, motion and contact evidence, controls, resource cadence and mission references. World must reject inconsistent candidates before replacement. Preserve all supported v4-v7 saves and prior installed profiles.

## Acceptance and limits

Review the complete source/tests/criteria/binding packet, run the joined core and desktop checks, and qualify a genuine host cold continuation with the designated executors. Full T02, actual player-hit acceptance and live parity are separate. No existing GitHub issue number is assigned to this source handoff.
