# Packs

The active pack set is the source of truth for every model and
runtime that the Studio uses.

## Listing

`Pack Manager` lists every installed pack with its id, version,
target, signature, size and measured target status. A pack that is
not signed is highlighted and never activated.

## Verify

`Pack Manager → verify` recomputes the SHA-256 of every file in
the pack and compares it to the pack lock. A mismatch is treated
as tamper and the pack is deactivated.

## Repair

`Pack Manager → repair from installer payload` copies the pack
bytes from the selected offline bundle root into the operator's
pack root. The repair is interrupted-safe: the previous pack
remains active until the new pack is fully signed and verified.

## Activate

`Pack Manager → activate` switches the active pack pointer to the
new pack. The old pack remains available for rollback.

## Rollback

`Pack Manager → rollback` switches the active pack pointer back to
the previous pack. The rollback is a one-step operation; every
rollback is recorded in the receipts.
