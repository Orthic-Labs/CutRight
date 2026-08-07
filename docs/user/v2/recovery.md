# Recovery

The Studio can recover from most local faults without operator
action.

## Automatic recovery

* A missing index is rebuilt.
* Abandoned staging directories are removed.
* An interrupted job is resumed.
* A tampered pack is replaced from the offline bundle.

## Manual recovery

Some faults require an operator decision. The Recovery mode lists
them:

* A canonical object whose hash does not match the receipt tree is
  treated as tampered. The version cannot be recovered silently;
  the operator chooses between restore from backup and relink.
* A migration that cannot complete because of an incompatible
  schema is blocked. The operator chooses the next step.
* A pack whose signature is missing is not activated. The operator
  must repair it from the offline bundle or remove it.

## Backups

Before a destructive operation (migration, pack activation, project
repair), the Studio writes a backup. The backup path is shown in the
mode that performs the operation and is recorded in the receipts.
