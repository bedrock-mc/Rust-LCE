# Audio Bank Parity Notes

This document captures the current parity-grounded state for legacy LCE audio banks and the exact next steps required for 1:1 behavior.

## Legacy sources

- Core bank candidates are staged from `LCE-Original/Minecraft.Client/Common/res/audio`:
  - `Minecraft.xgs`
  - `minecraft.xsb`
  - `resident.xwb`
  - `streamed.xwb`
- Title update bank candidates are staged from `LCE-Original/Minecraft.Client/Common/res/TitleUpdate/audio`:
  - `additional.xsb`
  - `additional.xwb`
  - `AdditionalMusic.xwb`
- UI menu bank candidates are staged from `LCE-Original/Minecraft.Client/Common/Media/Sound/Xbox`:
  - `MenuSounds.xgs`
  - `MenuSounds.xsb`
  - `MenuSounds.xwb`

## Probe findings

Using `cargo run --bin xact_bank_probe -- <path-to-xwb>`:

- `resident.xwb`: `DNBW`, big-endian, version `46`, `entry_count=279`, `entry_meta_size=24`.
- `streamed.xwb`: `DNBW`, big-endian, version `46`, `entry_count=24`, `entry_meta_size=24`.
- `MenuSounds.xwb`: `DNBW`, big-endian, version `46`, `entry_count=6`, `entry_meta_size=24`.
- Entry mini-format decodes consistently show `format_tag=1`, which is XMA in XACT miniwave format.

## Why full 1:1 playback is still pending

- Current Bevy audio path loads playable loose assets (`wav`) but does not decode XMA from XWB banks.
- Most legacy gameplay/UI cues in staged banks are XMA-coded, so parity event wiring alone is not enough without decode support.

## 1:1 completion path

1. Add deterministic XWB extraction path for XMA entries (decode to PCM WAV, preserving cue/index mapping).
2. Add XSB cue-table parsing so cue names map to exact wave entries the same way as the original engine.
3. Route gameplay/UI/music events through cue IDs anchored to `SoundNames.cpp` + `SoundTypes.h`.
4. Keep loose WAV fallback only for bring-up; remove fallback once bank decode path is stable.
