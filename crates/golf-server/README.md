# golf-server

The network service around `golf-shot`:

- **Course cache:** loads a course when a round is created; one `Arc<Course>` per course, shared by every round on it.
- **Sessions:** per round, the ball positions, and the wind state. The server is the authority
  (D4).
- **Connections:** phone uplink (P1 payloads, on an open WebSocket) and display downlink ("swing received" at once,
  then the `ShotResult`). Timestamps on every message for the latency measurements.

Design and budgets: [02-latency-budget.md](../../docs/02-latency-budget.md). Transport and encoding are proposals
until M8.

**Depends on:** `golf-core`, `golf-course`, `golf-shot`; later `tokio` and a WebSocket stack. **Status:** scaffold
(M0); M8.
