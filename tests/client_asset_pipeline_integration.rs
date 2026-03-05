use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::client::asset_pipeline::{
    default_additional_music_xwb_candidates, default_additional_xsb_candidates,
    default_additional_xwb_candidates, default_back_sound_candidates,
    default_click_sound_candidates, default_clouds_candidates, default_font_candidates,
    default_gui_candidates, default_icons_candidates, default_inventory_candidates,
    default_items_candidates, default_menu_sounds_xgs_candidates,
    default_menu_sounds_xsb_candidates, default_menu_sounds_xwb_candidates,
    default_minecraft_xgs_candidates, default_minecraft_xsb_candidates,
    default_mojangles_font_candidates, default_player_skin_candidates,
    default_pop_sound_candidates, default_resident_xwb_candidates, default_streamed_xwb_candidates,
    default_terrain_candidates, default_wood_click_sound_candidates, stage_default_runtime_assets,
    stage_terrain_texture,
};

#[test]
fn stages_first_existing_terrain_texture_to_runtime_assets() {
    let base = unique_temp_directory("asset_stage");
    let source = base.join("source_terrain.png");
    std::fs::create_dir_all(&base).expect("temp dir should be created");
    std::fs::write(&source, [1_u8, 2, 3, 4]).expect("fake texture should be written");

    let manifest = stage_terrain_texture(&base, std::slice::from_ref(&source))
        .expect("asset staging should succeed");

    assert_eq!(
        manifest.terrain_texture_asset_path.as_deref(),
        Some("runtime/terrain.png")
    );
    assert_eq!(
        manifest
            .terrain_texture_source_path
            .as_ref()
            .expect("source path should be present"),
        &source
    );

    let staged = base.join("assets").join("runtime").join("terrain.png");
    let bytes = std::fs::read(staged).expect("staged texture should exist");
    assert_eq!(bytes, vec![1_u8, 2, 3, 4]);

    cleanup_dir(&base);
}

#[test]
fn returns_empty_manifest_when_no_candidate_exists() {
    let base = unique_temp_directory("asset_stage_missing");
    std::fs::create_dir_all(&base).expect("temp dir should be created");

    let missing = base.join("missing.png");
    let manifest = stage_terrain_texture(&base, std::slice::from_ref(&missing))
        .expect("staging should succeed");

    assert!(manifest.terrain_texture_asset_path.is_none());
    assert!(manifest.terrain_texture_source_path.is_none());

    cleanup_dir(&base);
}

#[test]
fn default_candidates_include_common_legacy_paths() {
    let base = PathBuf::from("C:/workspace/LCE-Rust/lce_rust");
    let candidates = default_terrain_candidates(&base);
    let gui_candidates = default_gui_candidates(&base);
    let cloud_candidates = default_clouds_candidates(&base);
    let inventory_candidates = default_inventory_candidates(&base);
    let items_candidates = default_items_candidates(&base);
    let icon_candidates = default_icons_candidates(&base);
    let font_candidates = default_font_candidates(&base);
    let mojangles_font_candidates = default_mojangles_font_candidates(&base);
    let click_candidates = default_click_sound_candidates(&base);
    let back_candidates = default_back_sound_candidates(&base);
    let pop_candidates = default_pop_sound_candidates(&base);
    let wood_click_candidates = default_wood_click_sound_candidates(&base);
    let skin_candidates = default_player_skin_candidates(&base);
    let minecraft_xgs_candidates = default_minecraft_xgs_candidates(&base);
    let minecraft_xsb_candidates = default_minecraft_xsb_candidates(&base);
    let resident_xwb_candidates = default_resident_xwb_candidates(&base);
    let streamed_xwb_candidates = default_streamed_xwb_candidates(&base);
    let additional_xsb_candidates = default_additional_xsb_candidates(&base);
    let additional_xwb_candidates = default_additional_xwb_candidates(&base);
    let additional_music_xwb_candidates = default_additional_music_xwb_candidates(&base);
    let menu_sounds_xgs_candidates = default_menu_sounds_xgs_candidates(&base);
    let menu_sounds_xsb_candidates = default_menu_sounds_xsb_candidates(&base);
    let menu_sounds_xwb_candidates = default_menu_sounds_xwb_candidates(&base);

    assert!(
        candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/terrain.png")
        }),
        "common legacy terrain path should be included"
    );

    assert!(
        candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("assets/Common/res/terrain.png")
        }),
        "local assets Common terrain path should be included"
    );

    assert!(
        candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("assets/Commons/res/terrain.png")
        }),
        "local assets Commons terrain path should be included"
    );

    assert!(
        gui_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/gui/gui.png")
        }),
        "common legacy gui path should be included"
    );

    assert!(
        cloud_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/environment/clouds.png")
        }),
        "common legacy clouds path should be included"
    );

    assert!(
        inventory_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/gui/inventory.png")
        }),
        "common legacy inventory path should be included"
    );

    assert!(
        items_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/gui/items.png")
        }),
        "common legacy items path should be included"
    );

    assert!(
        icon_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/gui/icons.png")
        }),
        "common legacy icon path should be included"
    );

    assert!(
        font_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/font/default.png")
        }),
        "common legacy font path should be included"
    );

    assert!(
        mojangles_font_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/font/Mojangles.ttf")
        }),
        "common legacy mojangles ttf path should be included"
    );

    assert!(
        mojangles_font_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("assets/Common/Media/font/Mojangles.ttf")
        }),
        "local assets Common mojangles ttf path should be included"
    );

    assert!(
        click_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/click.wav")
        }),
        "common legacy click sound path should be included"
    );

    assert!(
        click_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("assets/Common/Media/Sound/click.wav")
        }),
        "local assets Common click sound path should be included"
    );

    assert!(
        back_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/btn_Back.wav")
        }),
        "common legacy back sound path should be included"
    );

    assert!(
        pop_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/pop.wav")
        }),
        "common legacy pop sound path should be included"
    );

    assert!(
        wood_click_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/wood click.wav")
        }),
        "common legacy wood click sound path should be included"
    );

    assert!(
        skin_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/mob/char.png")
        }),
        "common legacy player skin path should be included"
    );

    assert!(
        minecraft_xgs_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/audio/Minecraft.xgs")
        }),
        "common legacy minecraft xgs path should be included"
    );

    assert!(
        minecraft_xsb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/audio/minecraft.xsb")
        }),
        "common legacy minecraft xsb path should be included"
    );

    assert!(
        minecraft_xsb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("assets/Common/res/audio/minecraft.xsb")
        }),
        "local assets Common minecraft xsb path should be included"
    );

    assert!(
        resident_xwb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/audio/resident.xwb")
        }),
        "common legacy resident xwb path should be included"
    );

    assert!(
        streamed_xwb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/audio/streamed.xwb")
        }),
        "common legacy streamed xwb path should be included"
    );

    assert!(
        additional_xsb_candidates.iter().any(|path| {
            path.to_string_lossy().replace('\\', "/").contains(
                "LCE-Original/Minecraft.Client/Common/res/TitleUpdate/audio/additional.xsb",
            )
        }),
        "common legacy additional xsb path should be included"
    );

    assert!(
        additional_xwb_candidates.iter().any(|path| {
            path.to_string_lossy().replace('\\', "/").contains(
                "LCE-Original/Minecraft.Client/Common/res/TitleUpdate/audio/additional.xwb",
            )
        }),
        "common legacy additional xwb path should be included"
    );

    assert!(
        additional_music_xwb_candidates.iter().any(|path| {
            path.to_string_lossy().replace('\\', "/").contains(
                "LCE-Original/Minecraft.Client/Common/res/TitleUpdate/audio/AdditionalMusic.xwb",
            )
        }),
        "common legacy additional music xwb path should be included"
    );

    assert!(
        menu_sounds_xgs_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/Xbox/MenuSounds.xgs")
        }),
        "common legacy menu sounds xgs path should be included"
    );

    assert!(
        menu_sounds_xsb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/Xbox/MenuSounds.xsb")
        }),
        "common legacy menu sounds xsb path should be included"
    );

    assert!(
        menu_sounds_xwb_candidates.iter().any(|path| {
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/Media/Sound/Xbox/MenuSounds.xwb")
        }),
        "common legacy menu sounds xwb path should be included"
    );
}

#[test]
fn local_common_candidates_are_prioritized_before_legacy_paths() {
    let base = PathBuf::from("C:/workspace/LCE-Rust/lce_rust");

    let terrain_candidates = default_terrain_candidates(&base)
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();

    let local_index = terrain_candidates
        .iter()
        .position(|path| path.contains("assets/Common/res/terrain.png"))
        .expect("local common terrain path should be present");
    let legacy_index = terrain_candidates
        .iter()
        .position(|path| path.contains("LCE-Original/Minecraft.Client/Common/res/terrain.png"))
        .expect("legacy terrain path should be present");
    let consoles_index = terrain_candidates
        .iter()
        .position(|path| {
            path.contains("MinecraftConsoles-main/Minecraft.Client/Common/res/terrain.png")
        })
        .expect("MinecraftConsoles terrain path should be present");

    assert!(
        local_index < legacy_index,
        "local Common candidates should be preferred before legacy relative paths"
    );
    assert!(
        consoles_index < legacy_index,
        "MinecraftConsoles candidates should be preferred before legacy relative paths"
    );
}

#[test]
fn stages_runtime_ui_and_audio_assets_when_present() {
    let base = unique_temp_directory("asset_stage_full");

    let runtime_ui = base.join("assets").join("runtime").join("ui");
    let runtime_environment = base.join("assets").join("runtime").join("environment");
    let runtime_font = base.join("assets").join("runtime").join("font");
    let runtime_mob = base.join("assets").join("runtime").join("mob");
    let runtime_audio = base.join("assets").join("runtime").join("audio");
    let runtime_audio_banks = runtime_audio.join("banks");
    std::fs::create_dir_all(&runtime_ui).expect("runtime ui dir should be created");
    std::fs::create_dir_all(&runtime_environment)
        .expect("runtime environment dir should be created");
    std::fs::create_dir_all(&runtime_font).expect("runtime font dir should be created");
    std::fs::create_dir_all(&runtime_mob).expect("runtime mob dir should be created");
    std::fs::create_dir_all(&runtime_audio).expect("runtime audio dir should be created");
    std::fs::create_dir_all(&runtime_audio_banks)
        .expect("runtime audio bank dir should be created");

    std::fs::write(
        base.join("assets").join("runtime").join("terrain.png"),
        [7_u8, 7, 7],
    )
    .expect("terrain test file should be written");
    std::fs::write(runtime_environment.join("clouds.png"), [24_u8, 24, 24])
        .expect("clouds test file should be written");
    std::fs::write(runtime_ui.join("gui.png"), [8_u8, 8, 8])
        .expect("gui test file should be written");
    std::fs::write(
        runtime_ui.join("inventory.png"),
        minimal_png_header_bytes(256, 256),
    )
    .expect("inventory test file should be written");
    std::fs::write(runtime_ui.join("items.png"), [12_u8, 12, 12])
        .expect("items test file should be written");
    std::fs::write(runtime_ui.join("icons.png"), [9_u8, 9, 9])
        .expect("icons test file should be written");
    std::fs::write(runtime_font.join("default.png"), [11_u8, 11, 11])
        .expect("font test file should be written");
    std::fs::write(runtime_font.join("Mojangles.ttf"), [25_u8, 25, 25])
        .expect("mojangles font test file should be written");
    std::fs::write(runtime_mob.join("char.png"), [13_u8, 13, 13])
        .expect("player skin test file should be written");
    std::fs::write(runtime_audio.join("click.wav"), minimal_pcm_wav_bytes())
        .expect("click test file should be written");
    std::fs::write(runtime_audio.join("btn_back.wav"), minimal_pcm_wav_bytes())
        .expect("back test file should be written");
    std::fs::write(runtime_audio.join("pop.wav"), minimal_pcm_wav_bytes())
        .expect("pop test file should be written");
    std::fs::write(
        runtime_audio.join("wood_click.wav"),
        minimal_pcm_wav_bytes(),
    )
    .expect("wood click test file should be written");
    std::fs::write(runtime_audio_banks.join("minecraft.xgs"), [14_u8, 14, 14])
        .expect("minecraft xgs test file should be written");
    std::fs::write(runtime_audio_banks.join("minecraft.xsb"), [15_u8, 15, 15])
        .expect("minecraft xsb test file should be written");
    std::fs::write(runtime_audio_banks.join("resident.xwb"), [16_u8, 16, 16])
        .expect("resident xwb test file should be written");
    std::fs::write(runtime_audio_banks.join("streamed.xwb"), [17_u8, 17, 17])
        .expect("streamed xwb test file should be written");
    std::fs::write(runtime_audio_banks.join("additional.xsb"), [18_u8, 18, 18])
        .expect("additional xsb test file should be written");
    std::fs::write(runtime_audio_banks.join("additional.xwb"), [19_u8, 19, 19])
        .expect("additional xwb test file should be written");
    std::fs::write(
        runtime_audio_banks.join("additional_music.xwb"),
        [20_u8, 20, 20],
    )
    .expect("additional music xwb test file should be written");
    std::fs::write(runtime_audio_banks.join("menusounds.xgs"), [21_u8, 21, 21])
        .expect("menu sounds xgs test file should be written");
    std::fs::write(runtime_audio_banks.join("menusounds.xsb"), [22_u8, 22, 22])
        .expect("menu sounds xsb test file should be written");
    std::fs::write(runtime_audio_banks.join("menusounds.xwb"), [23_u8, 23, 23])
        .expect("menu sounds xwb test file should be written");

    let manifest =
        stage_default_runtime_assets(&base).expect("default runtime staging should succeed");

    assert_eq!(
        manifest.terrain_texture_asset_path.as_deref(),
        Some("runtime/terrain.png")
    );
    assert_eq!(
        manifest.gui_texture_asset_path.as_deref(),
        Some("runtime/ui/gui.png")
    );
    assert_eq!(
        manifest.clouds_texture_asset_path.as_deref(),
        Some("runtime/environment/clouds.png")
    );
    assert_eq!(
        manifest.inventory_texture_asset_path.as_deref(),
        Some("runtime/ui/inventory.png")
    );
    assert_eq!(
        manifest.items_texture_asset_path.as_deref(),
        Some("runtime/ui/items.png")
    );
    assert_eq!(
        manifest.icons_texture_asset_path.as_deref(),
        Some("runtime/ui/icons.png")
    );
    assert_eq!(
        manifest.font_texture_asset_path.as_deref(),
        Some("runtime/font/default.png")
    );
    assert_eq!(
        manifest.mojangles_font_asset_path.as_deref(),
        Some("runtime/font/Mojangles.ttf")
    );
    assert_eq!(
        manifest.player_skin_texture_asset_path.as_deref(),
        Some("runtime/mob/char.png")
    );
    assert_eq!(
        manifest.click_sound_asset_path.as_deref(),
        Some("runtime/audio/click.wav")
    );
    assert_eq!(
        manifest.back_sound_asset_path.as_deref(),
        Some("runtime/audio/btn_back.wav")
    );
    assert_eq!(
        manifest.pop_sound_asset_path.as_deref(),
        Some("runtime/audio/pop.wav")
    );
    assert_eq!(
        manifest.wood_click_sound_asset_path.as_deref(),
        Some("runtime/audio/wood_click.wav")
    );
    assert_eq!(
        manifest.minecraft_xgs_asset_path.as_deref(),
        Some("runtime/audio/banks/minecraft.xgs")
    );
    assert_eq!(
        manifest.minecraft_xsb_asset_path.as_deref(),
        Some("runtime/audio/banks/minecraft.xsb")
    );
    assert_eq!(
        manifest.resident_xwb_asset_path.as_deref(),
        Some("runtime/audio/banks/resident.xwb")
    );
    assert_eq!(
        manifest.streamed_xwb_asset_path.as_deref(),
        Some("runtime/audio/banks/streamed.xwb")
    );
    assert_eq!(
        manifest.additional_xsb_asset_path.as_deref(),
        Some("runtime/audio/banks/additional.xsb")
    );
    assert_eq!(
        manifest.additional_xwb_asset_path.as_deref(),
        Some("runtime/audio/banks/additional.xwb")
    );
    assert_eq!(
        manifest.additional_music_xwb_asset_path.as_deref(),
        Some("runtime/audio/banks/additional_music.xwb")
    );
    assert_eq!(
        manifest.menu_sounds_xgs_asset_path.as_deref(),
        Some("runtime/audio/banks/menusounds.xgs")
    );
    assert_eq!(
        manifest.menu_sounds_xsb_asset_path.as_deref(),
        Some("runtime/audio/banks/menusounds.xsb")
    );
    assert_eq!(
        manifest.menu_sounds_xwb_asset_path.as_deref(),
        Some("runtime/audio/banks/menusounds.xwb")
    );

    cleanup_dir(&base);
}

#[test]
fn skips_invalid_click_sound_assets_to_avoid_decoder_panics() {
    let base = unique_temp_directory("asset_stage_invalid_click");

    let runtime_audio = base.join("assets").join("runtime").join("audio");
    std::fs::create_dir_all(&runtime_audio).expect("runtime audio dir should be created");
    std::fs::write(runtime_audio.join("click.wav"), [0_u8, 1, 2, 3, 4, 5])
        .expect("invalid click file should be written");

    let manifest =
        stage_default_runtime_assets(&base).expect("default runtime staging should succeed");

    assert!(manifest.click_sound_asset_path.is_none());
    assert!(manifest.click_sound_source_path.is_none());
    assert!(manifest.back_sound_asset_path.is_none());
    assert!(manifest.back_sound_source_path.is_none());
    assert!(manifest.pop_sound_asset_path.is_none());
    assert!(manifest.pop_sound_source_path.is_none());
    assert!(manifest.wood_click_sound_asset_path.is_none());
    assert!(manifest.wood_click_sound_source_path.is_none());
    assert!(manifest.minecraft_xgs_asset_path.is_none());
    assert!(manifest.minecraft_xsb_asset_path.is_none());
    assert!(manifest.resident_xwb_asset_path.is_none());
    assert!(manifest.streamed_xwb_asset_path.is_none());
    assert!(manifest.additional_xsb_asset_path.is_none());
    assert!(manifest.additional_xwb_asset_path.is_none());
    assert!(manifest.additional_music_xwb_asset_path.is_none());
    assert!(manifest.menu_sounds_xgs_asset_path.is_none());
    assert!(manifest.menu_sounds_xsb_asset_path.is_none());
    assert!(manifest.menu_sounds_xwb_asset_path.is_none());

    cleanup_dir(&base);
}

#[test]
fn skips_screenshot_sized_inventory_png_and_falls_back_to_legacy_candidate() {
    let base = unique_temp_directory("asset_stage_inventory_filter");
    let runtime_ui = base.join("assets").join("runtime").join("ui");
    std::fs::create_dir_all(&runtime_ui).expect("runtime ui dir should be created");

    std::fs::write(
        runtime_ui.join("inventory.png"),
        minimal_png_header_bytes(436, 376),
    )
    .expect("screenshot-sized inventory file should be written");

    let fallback_inventory = default_inventory_candidates(&base)
        .into_iter()
        .find(|candidate| {
            candidate
                .to_string_lossy()
                .replace('\\', "/")
                .contains("LCE-Original/Minecraft.Client/Common/res/gui/inventory.png")
        })
        .expect("legacy inventory candidate path should be present");

    let fallback_parent = fallback_inventory
        .parent()
        .expect("fallback inventory path should have a parent");
    std::fs::create_dir_all(fallback_parent).expect("fallback parent should be created");
    std::fs::write(&fallback_inventory, minimal_png_header_bytes(256, 256))
        .expect("legacy-sized inventory file should be written");

    let manifest =
        stage_default_runtime_assets(&base).expect("default runtime staging should succeed");

    assert_eq!(
        manifest.inventory_texture_asset_path.as_deref(),
        Some("runtime/ui/inventory.png")
    );
    assert_eq!(
        manifest
            .inventory_texture_source_path
            .as_ref()
            .expect("inventory source should be present"),
        &fallback_inventory
    );

    let staged = runtime_ui.join("inventory.png");
    let staged_bytes = std::fs::read(staged).expect("staged inventory should exist");
    assert_eq!(staged_bytes, minimal_png_header_bytes(256, 256));

    cleanup_dir(&base);
    let fallback_root = base.join("..").join("..").join("LCE-Original");
    cleanup_dir(&fallback_root);
}

fn unique_temp_directory(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();

    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "lce_rust_{test_name}_{}_{}",
        std::process::id(),
        nanos
    ));
    dir.push("workspace");
    dir.push("project");
    dir
}

fn cleanup_dir(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}

fn minimal_pcm_wav_bytes() -> Vec<u8> {
    let sample_rate: u32 = 8_000;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let samples: [i16; 2] = [0, 0];

    let data_size = u32::try_from(samples.len() * std::mem::size_of::<i16>())
        .expect("data size should fit u32");
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample) / 8;
    let block_align = channels * bits_per_sample / 8;
    let riff_chunk_size = 36 + data_size;

    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    bytes
}

fn minimal_png_header_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes.extend_from_slice(&13_u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes
}
