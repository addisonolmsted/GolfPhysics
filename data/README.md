# data

Versioned tuning data, loaded at start-up (invariant 7: tuning is data, not code). A file is never
edited after a release; a change is a new version, and the versions in use go into every shot's request hash.

| Path | Milestone | What |
|---|---|---|
| `calibration/reference.v1.json` | M2 ✓ | TrackMan tour and amateur fit rows, never-fitted handicap sniff rows with bands, every source and what is verified |
| `aero/ball.v1.json` | M2 ✓ | fitted drag, lift and spin-decay coefficients, with the fit command and residuals; compiled into `golf-flight` |
| `materials/physics.v1.json` | M3 | per C0 class: bounce, roll and lie parameters, overriding the bundle's defaults (D18) |
| `clubs/standard.v1.json` | M4 | the club table: loft, length, D-plane coefficients, expected speeds |
| `trees/generic.v1.json` | M6 | generic tree and shrub models with tunable proxy detail, until the asset library exists (D20) |
| `assist/assist.v1.json` | M5, M7 | the one assist configuration: speed-curve bands, baseline carries, correction dials |
