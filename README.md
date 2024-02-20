# `bevy_eventwork_mod_websockets` (BEMW)

[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/quick-start/plugin-development/#main-branch-tracking)
[![crates.io](https://img.shields.io/crates/v/bevy_eventwork_mod_websockets)](https://crates.io/crates/bevy_eventwork_mod_websockets)
[![docs.rs](https://docs.rs/bevy_eventwork_mod_websockets/badge.svg)](https://docs.rs/bevy_eventwork_mod_websockets)

A crate that provides a websocket networking transport layer for [Bevy_eventwork](https://github.com/jamescarterbell/bevy_eventwork) that supports WASM and Native.

## Supported Platforms

- WASM
- Windows
- Linux
- Mac

## Getting Started

See [Bevy_eventwork](https://github.com/jamescarterbell/bevy_eventwork) for details on how to use `bevy_eventwork`.

The only difference from bevy_eventworks getting started directions is to use this crates `WebSocketProvider` and `NetworkSettings`.
Other than that the crate functions identically to stock bevy_eventworks. No features, changes, or manual shenanigans are needed to compile for WASM.
It just works.

```rust
    app.add_plugins(bevy_eventwork::EventworkPlugin::<
        WebSocketProvider,
        bevy::tasks::TaskPool,
    >::default());

    app.insert_resource(NetworkSettings::default());

```

## Supported Eventwork + Bevy Version

| EventWork Version | BEMW Version | Bevy Version |
| :---------------: | :----------: | :----------: |
|        0.8        |     0.1      |     0.13     |
