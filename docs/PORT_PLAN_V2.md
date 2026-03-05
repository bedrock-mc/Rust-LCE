# LCEMP Rust Port Plan V2

## Mission
- Port the full LCEMP game to Rust for Windows-first offline play.
- Keep gameplay and world behavior parity as the primary target.
- Defer multiplayer until after offline parity is complete.
- After parity, migrate networking to latest Bedrock packet model and then add RakNet.

## Scope Boundaries
- In scope now: runtime, world simulation, save/load, gameplay systems, rendering, UI, audio, input, Windows app shell.
- Out of scope now: LCEMP protocol compatibility, online session support, RakNet transport, Bedrock packet integration.
- Mandatory source parity target: full `Minecraft.World` subsystem coverage (no intentional omissions).

## Minecraft.World Parity Requirement (Mandatory)
- Port world generation stack end-to-end (noise, biome source/layers/cache, level sources, decorators/populators).
- Port world/chunk lifecycle and simulation (chunk loading/ticking, block/tile ticks, lighting, weather/time hooks).
- Port entity/mob/gameplay data paths that live in `Minecraft.World` (entities, AI hooks, combat/state interactions).
- Port `Minecraft.World` save/data interfaces and formats (NBT, region/chunk storage, level metadata, conversions as needed).
- Treat representative classes/files as required parity anchors, including `RandomLevelSource`, `HellRandomLevelSource`, `TheEndLevelRandomLevelSource`, `BiomeSource`, `BiomeDecorator`, `PerlinNoise`, `SimplexNoise`, and `Level` pathways.

## Architecture Direction
- Build gameplay core as protocol-agnostic from day one.
- Isolate network/protocol behind adapter boundaries so Bedrock migration does not require gameplay rewrites.
- Keep deterministic simulation as a first-class requirement (fixed tick rate, seed-stable behavior).
- Use Bevy for the client engine layer (windowing, rendering, input, audio, app lifecycle) on Windows.
- Keep world simulation, save pipeline, and gameplay logic engine-agnostic in core crates so Bedrock/net migration later does not require Bevy-specific rewrites.

## Milestones

### M0 - Foundation
- Build runtime shell, fixed-step tick loop (20 TPS), and deterministic timing primitives.
- Implement startup/boot sequencing with explicit initialization ordering.
- Stand up initial integration harness for runtime and boot behavior.
- Integration coverage required for this milestone before moving on.

### M1 - Core Save/World Backbone
- Implement NBT and region/chunk save pipeline compatibility.
- Implement initial world creation/load and core level data pipeline.
- Add deterministic seed and round-trip save integration tests.
- Integration coverage required for this milestone before moving on.

### M2 - First Playable Offline Slice
- Spawn into a world, move, break/place blocks, and persist changes.
- Support create/load/save loop for single-player flow.
- Add end-to-end integration scenarios for core play loop.
- Integration coverage required for this milestone before moving on.

### M3 - Gameplay Systems Parity
- Inventory, crafting, combat, damage, death/respawn, weather/time progression, mob AI slices.
- Add regression integration suites for each system added.
- Integration coverage required for this milestone before moving on.

### M4 - Client Parity (Windows)
- Rendering paths (terrain/entities/effects), UI screens/HUD, audio behavior, KBM/controller input parity.
- Integrate original game asset pipelines (textures, models/geometry data where applicable, UI resources, audio assets).
- Add integration and smoke tests for user flows and stability.
- M4 is not considered complete until gameplay UI is functional (dynamic hotbar, inventory screen flow, crafting UI, and HUD state wiring), not just backend systems.
- Integration coverage required for this milestone before moving on.

### M5 - Stabilization and Freeze
- Performance and bug-fix pass, long-run stability checks, parity sign-off.
- Lock offline parity baseline.
- Integration coverage required for this milestone before moving on.

### M6 - Post-Parity Networking Modernization (Later)
- Introduce latest Bedrock packet definitions via Axolotl/Valentine.
- Build gameplay-to-protocol adapters.
- Add RakNet transport and multiplayer flows after packet migration is stable.

## Integration Test Policy (Hard Rule)
- Every implementation step must add or update integration tests.
- Every bug fix must include a regression integration test.
- A step is not complete until integration tests pass for that step.
- Coverage priority is scenario correctness, determinism, and persistence integrity.

## Tracking Policy (Hard Rule)
- `docs/PROGRESS_TRACKER.md` is the source of truth for active step tracking.
- `docs/PARITY_MATRIX.md` is the source of truth for C++ class/file parity coverage and blockers.
- Update tracker at step start (mark in progress) and at step completion (mark done).
- Record tests added for each completed step.
- Do not close a step until tracker and tests are both updated.

## Current Execution Start
- M0.1 through M0.3 are complete.
- M1.1 through M1.3 implementations are in place with integration coverage.
- Engine direction is now locked to Bevy for client/runtime integration.
- Full `cargo test` execution is now passing with MSVC tooling configured.
- M2.1 through M2.3 are complete with integration coverage.
- M3.1 inventory and hotbar baseline is complete with integration coverage.
- M4.0 Bevy vertical slice app shell is complete (`bevy_client` bin + interaction path).
- M4.1 mouse-look and targeting polish is complete (including A/D inversion fix).
- M4.2 chunk visualization path is complete (chunk-window streaming + dynamic chunk load/unload).
- M4.3 original asset integration path is complete (terrain/gui/icons/click-audio staging + runtime wiring).
- M4.5 gameplay UI parity completion pass is reopened by request (backend inventory/crafting/death systems already exist; client UI wiring is now being finished).
  - M4.5.1 Dynamic hotbar HUD bound to live inventory state is complete.
  - M4.5.2 Inventory screen + cursor handoff/capture flow is complete.
  - M4.5.3 Crafting UI wiring against existing recipe backend is complete.
  - M4.5.4 Health/death/respawn HUD + pause/menu baseline is complete.
  - M4.5.5 Creative inventory UI parity baseline (in progress: tab-order + selector-page + hotbar-placement scaffold wired).
- M3.2 crafting and item use baseline is complete (recipes + item-use consumption flow).
- M3.3 combat/health/death-respawn baseline is complete.
- WG.1 deterministic worldgen scaffolding baseline is complete (`PerlinNoise`/`SimplexNoise`, `BiomeSource`/`BiomeDecorator`, and level source shells).
- WG.2.1 biome layer/cache baseline is complete (`BiomeSource` chunk cache semantics + biome-index block APIs + cache decay coverage).
- WG.2.2 decorator/populator parity baseline is complete (count-based decorator rules + runtime chunk generation fallback when persisted chunk payload is missing).
- WG.2.3 seeded reference fixture comparison is complete (`tests/worldgen_reference_fixture_integration.rs` fixture matrix and parity harness).
- WL.1 lifecycle scaffold baseline is complete (`ChunkLifecycleController` + lifecycle event stream + weather/time hooks + integration tests).
- WL.2 runtime lifecycle integration is complete (chunk streaming load/unload hooks + fixed-tick lifecycle progression wired in Bevy client runtime).
- WL.3 deterministic block/tile tick queue scaffold is complete (`ScheduledTick` queue in lifecycle controller + due-order integration tests).
- WL.4 lifecycle hook/runtime bridge is complete (lighting/weather runtime hook consumption + sky response + async world worker thread for off-main-thread chunk generation).
- EM.1 entities/mobs/data-path parity baseline scaffolding is complete (entity world registry, deterministic mob tick scaffolding, and damage/death hooks wired into offline session flow).
- GP.1 fluids tick/scheduling baseline scaffold is complete (`world::fluids` water/lava scheduling helpers + runtime scheduled-tick consumption hooks).
- GP.2 redstone-like logic baseline scaffold is complete (`world::redstone` component tick model + block-change scheduling + runtime scheduled-tick consumption hooks).
- GP.3.1 player solid-block collision/physics baseline is complete (AABB collision resolution in simulation + Bevy fixed-tick integration against loaded block state).
- GP.3.2a movement/input edge-case baseline is complete (player-overlap placement guard, jump impulse parity tuning, and creative double-tap flight toggle semantics alignment).
- GP.3.2b lifecycle scheduled-tick corner-case baseline is complete (duplicate `(kind, block, payload)` schedule dedupe + earlier re-schedule promotion semantics).
- GP.3.2b follow-up lifecycle/decorator edge-case pass is complete (chunk unload now purges stale pending/triggered ticks + chunk tick counters; decorator invariants enforced for valid world-coordinate bounds).
- GP.3.2b decorator parity follow-up is complete (`BiomeDecorator` placement window now follows C++ `+8` X/Z semantics and water/lava spring depths use nested random ranges from `Minecraft.World/BiomeDecorator.cpp`; surface placement height lookup now samples by world coordinate).
- GP.3.2b biome-specific decorator parity follow-up is complete (`BiomeDecorator`/`DesertBiome` extras now include pumpkin gate, desert-well gate, heightmap-vs-top-solid placement semantics, waterlily descent-style Y selection, and brown-mushroom heightmap attempts).
- GP.3.2b ore decorator parity follow-up is complete (`BiomeDecorator::decorateOres` attempt counts/depth distributions and ore coordinate-window semantics now match `Minecraft.World/BiomeDecorator.cpp`, including lapis average-depth sampling).
- GP.3.2b nether/end decorator-source parity follow-up is complete (`HellRandomLevelSource` now emits biome-decorator outputs, and `TheEndLevelRandomLevelSource` now emits `TheEndBiomeDecorator` chunk-triggered spike/podium/dragon markers while preserving End ore-pass behavior).
- GP.3.2b fluid simulation parity follow-up is complete (`LiquidTile` dynamic/static update flow is now ported with depth metadata, downhill/slope spread, decay, static-water reactivation, lava-water conversion, and block-change neighbor fluid scheduling).
- GP.3.2b fluid visual/interaction parity follow-up is complete (fluid mesh top/side shaping and UV behavior now tracks `TileRenderer::tesselateWaterInWorld`/`getWaterHeight` semantics, interaction placement can replace fluid cells, and fluid rendering now uses a merged global transparent fluid pass to avoid chunk-edge transparency seams).
- GP.3.2c terrain rendering edge-case pass is complete (nearest atlas sampling + side-face UV orientation fix + UV inset bleed mitigation).
- GP.3.2c grass-top shading follow-up is complete (top-face tint aligned to LCE colour-table grass common value `0x7cbd6b`).
- M4.5 visual parity follow-up now includes a `LevelRenderer::renderClouds`-anchored baseline (legacy `environment/clouds.png` staging + camera-relative cloud layer drift).
- Bevy client default chunk window is currently tuned to `CHUNK_LOAD_RADIUS=16` (override via `LCE_CHUNK_LOAD_RADIUS`), with an independent mesh staging cap via `LCE_CHUNK_MESH_RADIUS` (default `8`, temporary hard cap `10`) and a total per-frame mesh rebuild budget (`LCE_MAX_MESH_REBUILDS_PER_FRAME`, default `1`) to reduce streaming hitches/VRAM pressure while parity-debt work continues.
- Runtime testing defaults currently include daytime boot alignment (`day_time=6000`) and creative-style double-tap jump flight toggle to speed parity verification passes.
- Current active step: `GP.3` parity debt backlog closure (chunk mesh/update parity + save-flush stabilization resumed while `M4.5.5` remains open).
