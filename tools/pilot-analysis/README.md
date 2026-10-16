# Pilot analysis (first pass)

Python scripts used on the first SwingLab session (2026-10-16). Each takes an unzipped export folder (the folder that
holds `<session id>/`). They become `golf-lab` tools once the pilot format settles.

| Script | What it shows |
|---|---|
| `analyze.py <dir>` | stream integrity (rates, gaps, ordering), every event in time order, swing files against events |
| `clip.py <dir>` | accelerometer and gyro clipping per block; the unarmed-swing events |
| `address.py <dir>` | per arm: how long the phone sat at address and which stillness test held it up |
| `residual.py <dir>` | CoreMotion's user acceleration against the raw accelerometer on a still phone, by time since the last swing |
| `replay.py <dir>` | the old and new stillness rules replayed over the stream |
| `twist.py <dir>` | the phone's twist about its long axis at impact, per block (fade against draw) |
| `gyro.py <dir> <out.json>` | raw-gyro orientation against CoreMotion's per swing, and both against gravity at the next stillness; the data for `gyro-check-template.html` (replace `/*__DATA__*/null` with the JSON) |
