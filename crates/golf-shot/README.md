# golf-shot

The one entry point the server calls:

```rust
pub fn simulate(course: &Course, data: &EngineData, req: &ShotRequest) -> Result<ShotResult, ShotError>;
```

It runs swing → assist → impact → assist → flight → ground → rest facts, hashes the request and the result, and
assembles the `ShotResult` ([P2](../../docs/contracts/P2-shot-result.md)). `EngineData` holds the loaded data files
(clubs, aero, materials, the assist configuration) with their versions, which go into the request hash.

Also the home of the benchmarks (`cargo bench`), held to the [latency budget](../../docs/02-latency-budget.md).

**Depends on:** every step crate. **Status:** scaffold (M0).
