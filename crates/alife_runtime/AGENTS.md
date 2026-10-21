# alife_runtime

Own the shared neural session, backend lifetime, durable checkpoints, and sealed
capture/restore boundaries. Keep consumer orchestration, Bevy/UI, world legality,
and presentation outside this crate.

Preserve full acquired/private state and passive statistics on exact resume.
Device loss or unavailable hardware stops gameplay neural actions; offline native
CPU training and handoff remain explicit, separate operations.
