# Moat Task Lease Heartbeat Reap Implementation Plan

Goal: Add local task claim leases, heartbeat renewal, and stale-claim reaping so the moat-loop task graph can recover from crashed autonomous agents.

Implemented local deterministic external-controller coordination only. The CLI/runtime persist lease metadata in the local moat history file and use the existing adjacent lock file for mutation safety. This is not a daemon, crawler, or PR automation system.

Runtime behaviors:
- `MoatTaskNode` includes optional `claimed_at`, `lease_expires_at`, and `last_heartbeat_at` fields with serde defaults for legacy histories.
- Ready task claims can set an assigned agent and lease expiry.
- Heartbeats extend the lease for the assigned in-progress task and reject mismatched agents.
- Expired in-progress task leases can be reaped back to ready, clearing assignment and lease metadata.
- Completing, blocking, releasing, and reaping clear lease metadata.

CLI behaviors:
- `moat claim-task` and `moat dispatch-next` accept `--lease-seconds` (default 900).
- `moat heartbeat-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--lease-seconds N]` renews an in-progress task lease.
- `moat reap-stale-tasks --history-path PATH [--round-id ROUND_ID] [--now RFC3339]` requeues expired local claims.
