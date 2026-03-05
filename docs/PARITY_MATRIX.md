# LCEMP C++ to Rust Parity Matrix

Last updated: 2026-03-04

This file is the class/file parity map for the 1:1 port target. Use it with `docs/PORT_PLAN_V2.md` and `docs/PROGRESS_TRACKER.md`.

Status legend:
- `green`: parity-complete or very close for current scope
- `yellow`: baseline exists but still simplified vs C++
- `red`: missing or only stubbed

## Scale Snapshot

- LCEMP C++ file counts: `Minecraft.World` 715 `.cpp` + 852 `.h`, `Minecraft.Client` 615 `.cpp` + 703 `.h`
- LCEMP C++ line count (snapshot): ~442,012 lines (`.cpp` + `.h`)
- Rust line count (snapshot): ~16,334 lines (`src/**/*.rs`)
- Interpretation: current Rust code is still an early-stage subset; strict subsystem-by-subsystem parity tracking is required.

## Coverage Matrix

| Subsystem | C++ anchors | Rust anchors | Status | Notes |
| --- | --- | --- | --- | --- |
| Runtime tick/bootstrap | `Minecraft` startup + fixed 20 TPS loop | `src/runtime/fixed_step.rs`, `src/runtime/bootstrap.rs` | green | Deterministic fixed-step scaffold is stable.
| NBT + region IO | `NbtIo`, `RegionFile`, `McRegionChunkStorage` | `src/save/nbt.rs`, `src/save/region.rs`, `src/save/world_io.rs` | yellow | Works for current tests; format/data breadth is still narrower than full console payloads.
| Block/chunk world store | `LevelChunk`, chunk/block storage paths | `src/world/blocks.rs` | yellow | Solid baseline; chunk payload schema is simplified.
| Worldgen noise | `PerlinNoise`, `SimplexNoise` | `src/world/worldgen/noise.rs` | green | Seed stability fixtures exist.
| Biome source/cache/decorator | `BiomeSource`, `BiomeCache`, `BiomeDecorator` | `src/world/worldgen/biome.rs`, `src/world/worldgen/biome_cache.rs` | yellow | Main decorator rules ported; still output-marker based for many features.
| Level sources | `RandomLevelSource`, `HellRandomLevelSource`, `TheEndLevelRandomLevelSource` | `src/world/worldgen/level_source.rs` | yellow | Terrain/feature generation is deterministic but still simplified vs full C++ pipelines.
| Chunk lifecycle/tick queue | `Level` chunk/tick lifecycle | `src/world/lifecycle.rs`, runtime hooks in `src/bin/bevy_client.rs` | green | Deterministic scheduling and unload cleanup are in place.
| Fluids | `LiquidTile` update/scheduling + water movement branch in `LivingEntity::travel` | `src/world/fluids.rs`, `src/world/simulation.rs` | yellow | Fluid tick/update parity is broad; player swim drag/jump baseline is now present, with further medium-motion edge cases still pending.
| Redstone-like logic | redstone tile/component logic | `src/world/redstone.rs` | yellow | Baseline only (wire/torch/repeater semantics are simplified).
| Entities/mobs | `Entity`, `Mob`, AI goals | `src/world/entities.rs`, `src/world/simulation.rs` | red | Registry/tick scaffold exists; broad AI and gameplay interactions not yet ported.
| Inventory/crafting/item use | inventory/menu/item static ctor paths | `src/world/inventory.rs`, `src/world/crafting.rs`, `src/world/item_use.rs` | yellow | Core loop works; recipe/item coverage is small vs C++.
| Terrain rendering | `TileRenderer`, `LevelRenderer`, `ParticleEngine::destroy`, `TerrainParticle` | `src/client/terrain_meshing.rs`, `src/client/particles.rs`, `src/bin/bevy_client.rs` | yellow | Atlas/shape parity improved; break particles use bottom-face terrain tile sub-UV sampling (`getTexture(0,data)` intent), redstone wire now applies lit/unlit tint + connection geometry expansion, torch/lever now use thin non-cube meshes, and piston front-face texturing now follows block-data facing, but broader particle/shader/render-order parity is still pending.
| Gameplay UI | `Gui`, `InventoryScreen`, `CraftingScreen`, creative menu | `src/client/*_ui.rs`, `src/bin/bevy_client.rs` | yellow | Broad baseline exists; M4.5.5 still active. Menu labels prefer staged Mojangles TTF, creative tab/source lists now include late-tab parity additions (spawn eggs/horse armor/lead/map variants + transport/redstone late items), hotbar mouse-wheel selection is restored, sword right-click defend pose + hit-outline feedback are wired, and survival HUD now renders health + hunger icons plus XP bar/level from runtime player state.
| Clouds | `LevelRenderer::renderClouds`, `renderAdvancedClouds`, `createCloudMesh` | `src/client/clouds.rs`, `src/bin/bevy_client.rs` | yellow | Camera-relative formulas and staging are in place; mesh now uses a non-flat shell baseline (`h=4` intent) but full 4J advanced cloud face-visibility/chunk-list behavior is still pending.
| Audio banks/cues | XACT/XWB/XSB + cue routing | `src/client/asset_pipeline.rs`, `src/bin/xact_bank_probe.rs` | yellow | Bank staging/probing and WAV extraction exist; cue-accurate runtime playback still blocked by full decode/mapping.
| Networking | LCEMP protocol/session flows | (not implemented) | red | Intentionally deferred until offline parity lock.

## Immediate Port Queue

1. Expand save/chunk payload parity (`level.dat`, chunk payload schema, tile entities/entities fields).
2. Replace simplified gameplay registries with C++-anchored static ctor coverage (items, recipes, place/use mappings).
3. Port broader entity/mob behavior from `Minecraft.World` class-by-class, starting with core hostile/passive state machines.
4. Close M4.5.5 UI parity debt and remove temporary non-parity shortcuts.
5. Complete audio cue parity (XSB cue table + deterministic bank event routing).
