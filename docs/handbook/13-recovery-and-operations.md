# OMS Recovery And Operations

This chapter is the operational playbook for running the OMS pieces safely.

## Normal Startup

Recommended startup order:

1. load route config,
2. open durable execution journal,
3. replay journal into local state where applicable,
4. create adapter,
5. connect adapter,
6. recover venue open orders,
7. reconcile local and venue state,
8. enable strategy submissions.

Do not allow strategy submissions before recovery and reconciliation are
complete unless the route is explicitly configured for that risk.

## Crash Recovery

After process restart:

1. open journal,
2. replay command/event history,
3. rebuild expected local state,
4. reconnect provider,
5. request open orders,
6. compare with `reconcile_open_orders`,
7. emit restatements or trigger operator review.

If reconciliation finds `VenueOnly` orders, the venue has working orders the
local journal did not know about. Treat this as high severity.

If reconciliation finds `LocalOnly` orders, the local journal thinks something
is working but the venue does not report it. Treat this as a possible missed
terminal report.

## Disconnect Handling

Route policy determines behavior:

| Policy | Behavior |
| --- | --- |
| `Hold` | Keep current state and wait |
| `RejectNew` | Stop new orders, allow cancels |
| `CancelOpenOrders` | Attempt cancel of working orders |
| `Freeze` | Reject all strategy commands |

Choose policy per venue and strategy. A market-making strategy may prefer
cancel-on-disconnect; a passive research simulation may prefer hold.

## Kill Switch

There are two relevant kill-switch layers:

- `RiskLimits.kill_switch`: basic risk gate route kill.
- `RouteSafetyPolicy.kill_switch`: operational policy kill.

When a kill switch is active, decide whether cancels should still be allowed.
Most live systems allow cancels while blocking new risk.

## Journal Policy

`FileExecutionJournal::open(path, sync_on_write)` is the human-readable
append-only journal and supports two modes:

- `sync_on_write = true`: safer, slower, fsync-like behavior.
- `sync_on_write = false`: faster, less durable on power loss.

`WalExecutionJournal::open(WalJournalConfig::new(path))` is the binary WAL
option. It validates existing frames before accepting new writes, owns
monotonic WAL sequence assignment, detects checksum failures, and replays into
the same `JournalRecord` model as the text journal.

WAL sync policy is explicit:

- `WalSyncPolicy::EveryRecord`: safest and highest latency.
- `WalSyncPolicy::EveryNRecords(n)`: group commit by record count.
- `WalSyncPolicy::EveryDurationNs(ns)`: group commit by elapsed time.
- `WalSyncPolicy::Manual`: caller invokes `sync()`.
- `WalSyncPolicy::Never`: fastest, only appropriate for tests or externally
  replicated environments.
- `WalSyncPolicy::OnRiskBoundary`: sync around command/risk/recovery boundary
  records.

Production deployments should decide based on venue risk, account size, and
host filesystem behavior.

## Segmented WAL Policy

`SegmentedWalExecutionJournal::open(WalSegmentConfig::new(root))` is the
rotated binary WAL option for production-style OMS deployments. It stores
frames under a directory:

```text
execution-wal/
  manifest
  wal-000000000001.ofwal
  wal-000000000002.ofwal
```

Segment rotation is controlled by:

- `WalSegmentConfig::with_max_segment_bytes(bytes)`;
- `WalSegmentConfig::with_max_segment_records(records)`;
- explicit `SegmentedWalExecutionJournal::rotate_segment()`.

Rotation appends a `SegmentSeal` WAL frame to the old file and starts the next
segment id. The seal frame is part of the checksum and sequence chain, but it
does not replay as a command or execution event.

Recovery rules:

- scan segment files by numeric segment id;
- validate every frame checksum;
- validate `previous_checksum` links across segment boundaries;
- validate monotonic WAL sequence continuity;
- rebuild the manifest from segment bytes;
- fail closed on corrupt frames, checksum mismatches, or sequence gaps.

The `manifest` file is an operator inventory. It is not trusted as the source
of truth during recovery because the active segment can be newer than the last
manifest write under relaxed sync policies.

Low-latency guidance:

- use `WalSyncPolicy::EveryNRecords` or `EveryDurationNs` for group-commit
  behavior when the venue/account risk allows it;
- call `sync()` at explicit risk boundaries if using `Manual`;
- rotate by bytes to bound recovery and retention units;
- rotate by record count when predictable replay batch sizes matter;
- keep the WAL directory on low-latency local storage and archive sealed
  segments off the hot path.

## Checkpoint Policy

`FileExecutionCheckpointStore::open(CheckpointConfig::new(root))` provides the
first durable checkpoint store. It is additive to the journal APIs and does not
change engine runtime behavior.

Checkpoint save flow:

1. encode `ExecutionCheckpoint` with schema version and checksum,
2. write bytes to a `.tmp` file in the checkpoint directory,
3. flush the file,
4. call `sync_data()` when `CheckpointConfig::sync_on_save()` is enabled,
5. atomically rename the temp file to `.ofchk`,
6. sync the parent directory on Unix when sync-on-save is enabled.

The checkpoint records the last fully applied WAL sequence. Recovery tooling can
load the latest valid checkpoint and replay WAL records after that sequence.
Current checkpoint contents cover open orders, positions, route config hash,
kill-switch state, and checksum. Venue reconciliation must still run before
strategy submissions resume.

## Metrics To Export

At minimum export:

- submitted count,
- cancel count,
- amend count,
- events applied,
- risk rejections,
- adapter errors,
- recovered events,
- command queue depth,
- report queue depth,
- fanout drops,
- journal write errors,
- reconnect count,
- lifecycle state,
- health sequence.

## Alerting

High-priority alerts:

- adapter disconnected on live route,
- reconciliation not clean,
- fanout drops increasing,
- journal write failure,
- command queue full,
- report queue full,
- kill switch active unexpectedly,
- repeated venue rejects,
- cancel rejects on live working orders.

## Operator Workflow

When an incident occurs:

1. freeze or reject new commands,
2. preserve journal files,
3. capture adapter health,
4. request venue open orders,
5. reconcile,
6. cancel or restate according to policy,
7. restart strategy only after state is understood.

## Backtesting vs Live

Replay and simulation are deterministic tools. They do not prove live adapter
behavior. Live readiness also requires:

- provider conformance tests,
- reconnect tests,
- rate-limit tests,
- cancel/replace tests,
- journal replay tests,
- reconciliation tests.

## Release Checklist For OMS Changes

Before releasing OMS changes:

- run Rust tests with all features,
- run no-default-features tests,
- run clippy with warnings denied,
- run C ABI export check,
- run Python binding smoke,
- run Java binding smoke,
- verify docs coverage,
- verify changelog and handbook updates,
- confirm no API signatures were changed unexpectedly.
