# contracts

The seams between this engine and the systems around it. A contract change is a decision log entry plus a version
bump; the other side must agree first.

| Contract | Between | Status |
|---|---|---|
| [P0: course input](P0-course-input.md) | generator (C0 bundle) → `golf-course` | draft; reads C0 v4.2 as it is |
| [P1: swing input](P1-swing-input.md) | phone (SwingApp) → `golf-swing` | draft; SwingApp has no transport yet |
| [P2: shot result](P2-shot-result.md) | `golf-shot` → display and game layer | draft |

The generator's own contract, which P0 reads, is
[C0-delivery.md](../../../GolfProceduralGenerator/docs/contracts/C0-delivery.md). P0 doesn't restate C0; it says which
parts we use, how we interpret them, and what we add on top (our material overrides).

Versioning: each contract has a `version` integer in its wire form. Additive fields bump the minor note in the doc;
anything that changes meaning bumps the version, and the server accepts the previous version for one release.
