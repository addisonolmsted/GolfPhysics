# fixtures

The test courses are **not in git**. Tests and `golf-lab` read them from where the generator wrote them.

- **Location:** `$GOLF_FIXTURES`, defaulting to `/Volumes/GolfOut/out/physics_fixtures/v1`. That volume is a
  sparsebundle on the T7 drive; mount it with `hdiutil attach "/Volumes/T7 Shield/GolfOut.sparsebundle"`.
- **Contents:** six frozen C0 v4.2 bundles, each as a directory `course_4_<seed>/` and a tar, plus `SHA256SUMS`. The
  generator never edits them; a later generator change makes a new fixtures version (`v2`).
- Fixture tests skip with a message when the folder isn't there.

| Seed | Type | Why it's in the set |
|---|---|---|
| 7667961 | aeolian, typical | near the aeolian median; 30 bunkers, no water. **The default for smoke tests.** |
| 604486 | aeolian, typical | relief 28.5 m, 27 bunkers, no water |
| 5746282 | fluvial, typical | 20 bunkers, 1.1 ha of water in play, about 9,000 trees |
| 8337594 | fluvial, typical | 18 bunkers, 0.55 ha of water, about 8,900 trees |
| 1920628 | aeolian, rare | the most water: 33 ha of penalty area, 3 lakes |
| 7011100 | fluvial, rare | the most bunkers: 79, 3.6 ha of sand |

Source: the generator's [physics-starter.md](../../GolfProceduralGenerator/docs/physics-starter.md).

Our own small test data (recorded swings, golden requests) will live in `crates/*/tests/data/` and is in git.
