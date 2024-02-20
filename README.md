# `bevy_eventwork_mod_websockets` (BEMW)

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
