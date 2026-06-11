# alife_bevy_adapter

Bevy runtime adapter for A-Life.

This crate owns Bevy plugin wiring, ECS integration, rendering/debug adapters, and future playground hooks. Cognitive contracts belong in `alife_core`; world legality belongs in `alife_world` where it can stay testable without Bevy.
