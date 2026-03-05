use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeAssetManifest {
    pub terrain_texture_asset_path: Option<String>,
    pub terrain_texture_source_path: Option<PathBuf>,
    pub clouds_texture_asset_path: Option<String>,
    pub clouds_texture_source_path: Option<PathBuf>,
    pub gui_texture_asset_path: Option<String>,
    pub gui_texture_source_path: Option<PathBuf>,
    pub inventory_texture_asset_path: Option<String>,
    pub inventory_texture_source_path: Option<PathBuf>,
    pub creative_inventory_texture_asset_path: Option<String>,
    pub creative_inventory_texture_source_path: Option<PathBuf>,
    pub items_texture_asset_path: Option<String>,
    pub items_texture_source_path: Option<PathBuf>,
    pub icons_texture_asset_path: Option<String>,
    pub icons_texture_source_path: Option<PathBuf>,
    pub font_texture_asset_path: Option<String>,
    pub font_texture_source_path: Option<PathBuf>,
    pub mojangles_font_asset_path: Option<String>,
    pub mojangles_font_source_path: Option<PathBuf>,
    pub menu_logo_texture_asset_path: Option<String>,
    pub menu_logo_texture_source_path: Option<PathBuf>,
    pub player_skin_texture_asset_path: Option<String>,
    pub player_skin_texture_source_path: Option<PathBuf>,
    pub click_sound_asset_path: Option<String>,
    pub click_sound_source_path: Option<PathBuf>,
    pub back_sound_asset_path: Option<String>,
    pub back_sound_source_path: Option<PathBuf>,
    pub pop_sound_asset_path: Option<String>,
    pub pop_sound_source_path: Option<PathBuf>,
    pub wood_click_sound_asset_path: Option<String>,
    pub wood_click_sound_source_path: Option<PathBuf>,
    pub minecraft_xgs_asset_path: Option<String>,
    pub minecraft_xgs_source_path: Option<PathBuf>,
    pub minecraft_xsb_asset_path: Option<String>,
    pub minecraft_xsb_source_path: Option<PathBuf>,
    pub resident_xwb_asset_path: Option<String>,
    pub resident_xwb_source_path: Option<PathBuf>,
    pub streamed_xwb_asset_path: Option<String>,
    pub streamed_xwb_source_path: Option<PathBuf>,
    pub additional_xsb_asset_path: Option<String>,
    pub additional_xsb_source_path: Option<PathBuf>,
    pub additional_xwb_asset_path: Option<String>,
    pub additional_xwb_source_path: Option<PathBuf>,
    pub additional_music_xwb_asset_path: Option<String>,
    pub additional_music_xwb_source_path: Option<PathBuf>,
    pub menu_sounds_xgs_asset_path: Option<String>,
    pub menu_sounds_xgs_source_path: Option<PathBuf>,
    pub menu_sounds_xsb_asset_path: Option<String>,
    pub menu_sounds_xsb_source_path: Option<PathBuf>,
    pub menu_sounds_xwb_asset_path: Option<String>,
    pub menu_sounds_xwb_source_path: Option<PathBuf>,
    pub legacy_event_audio_asset_dir: Option<String>,
    pub legacy_event_audio_source_dir: Option<PathBuf>,
}

pub fn stage_default_runtime_assets(base_dir: &Path) -> io::Result<RuntimeAssetManifest> {
    let mut manifest = RuntimeAssetManifest::default();

    let terrain = stage_named_asset(
        base_dir,
        &default_terrain_candidates(base_dir),
        "runtime/terrain.png",
    )?;
    manifest.terrain_texture_asset_path = terrain.asset_path;
    manifest.terrain_texture_source_path = terrain.source_path;

    let clouds = stage_named_asset(
        base_dir,
        &default_clouds_candidates(base_dir),
        "runtime/environment/clouds.png",
    )?;
    manifest.clouds_texture_asset_path = clouds.asset_path;
    manifest.clouds_texture_source_path = clouds.source_path;

    let gui = stage_named_asset(
        base_dir,
        &default_gui_candidates(base_dir),
        "runtime/ui/gui.png",
    )?;
    manifest.gui_texture_asset_path = gui.asset_path;
    manifest.gui_texture_source_path = gui.source_path;

    let inventory = stage_named_asset_with_filter(
        base_dir,
        &default_inventory_candidates(base_dir),
        "runtime/ui/inventory.png",
        is_supported_inventory_texture_candidate,
    )?;
    manifest.inventory_texture_asset_path = inventory.asset_path;
    manifest.inventory_texture_source_path = inventory.source_path;

    let creative_inventory = stage_named_asset_with_filter(
        base_dir,
        &default_creative_inventory_candidates(base_dir),
        "runtime/ui/allitems.png",
        is_supported_inventory_texture_candidate,
    )?;
    manifest.creative_inventory_texture_asset_path = creative_inventory.asset_path;
    manifest.creative_inventory_texture_source_path = creative_inventory.source_path;

    let items = stage_named_asset(
        base_dir,
        &default_items_candidates(base_dir),
        "runtime/ui/items.png",
    )?;
    manifest.items_texture_asset_path = items.asset_path;
    manifest.items_texture_source_path = items.source_path;

    let icons = stage_named_asset(
        base_dir,
        &default_icons_candidates(base_dir),
        "runtime/ui/icons.png",
    )?;
    manifest.icons_texture_asset_path = icons.asset_path;
    manifest.icons_texture_source_path = icons.source_path;

    let font = stage_named_asset(
        base_dir,
        &default_font_candidates(base_dir),
        "runtime/font/default.png",
    )?;
    manifest.font_texture_asset_path = font.asset_path;
    manifest.font_texture_source_path = font.source_path;

    let mojangles_font = stage_named_asset(
        base_dir,
        &default_mojangles_font_candidates(base_dir),
        "runtime/font/Mojangles.ttf",
    )?;
    manifest.mojangles_font_asset_path = mojangles_font.asset_path;
    manifest.mojangles_font_source_path = mojangles_font.source_path;

    let menu_logo = stage_named_asset(
        base_dir,
        &default_menu_logo_candidates(base_dir),
        "runtime/ui/mclogo.png",
    )?;
    manifest.menu_logo_texture_asset_path = menu_logo.asset_path;
    manifest.menu_logo_texture_source_path = menu_logo.source_path;

    let player_skin = stage_named_asset(
        base_dir,
        &default_player_skin_candidates(base_dir),
        "runtime/mob/char.png",
    )?;
    manifest.player_skin_texture_asset_path = player_skin.asset_path;
    manifest.player_skin_texture_source_path = player_skin.source_path;

    let click_sound = stage_click_sound_asset(base_dir, &default_click_sound_candidates(base_dir))?;
    manifest.click_sound_asset_path = click_sound.asset_path;
    manifest.click_sound_source_path = click_sound.source_path;

    let back_sound = stage_named_wav_asset(
        base_dir,
        &default_back_sound_candidates(base_dir),
        "runtime/audio/btn_back.wav",
    )?;
    manifest.back_sound_asset_path = back_sound.asset_path;
    manifest.back_sound_source_path = back_sound.source_path;

    let pop_sound = stage_named_wav_asset(
        base_dir,
        &default_pop_sound_candidates(base_dir),
        "runtime/audio/pop.wav",
    )?;
    manifest.pop_sound_asset_path = pop_sound.asset_path;
    manifest.pop_sound_source_path = pop_sound.source_path;

    let wood_click_sound = stage_named_wav_asset(
        base_dir,
        &default_wood_click_sound_candidates(base_dir),
        "runtime/audio/wood_click.wav",
    )?;
    manifest.wood_click_sound_asset_path = wood_click_sound.asset_path;
    manifest.wood_click_sound_source_path = wood_click_sound.source_path;

    let minecraft_xgs = stage_named_asset(
        base_dir,
        &default_minecraft_xgs_candidates(base_dir),
        "runtime/audio/banks/minecraft.xgs",
    )?;
    manifest.minecraft_xgs_asset_path = minecraft_xgs.asset_path;
    manifest.minecraft_xgs_source_path = minecraft_xgs.source_path;

    let minecraft_xsb = stage_named_asset(
        base_dir,
        &default_minecraft_xsb_candidates(base_dir),
        "runtime/audio/banks/minecraft.xsb",
    )?;
    manifest.minecraft_xsb_asset_path = minecraft_xsb.asset_path;
    manifest.minecraft_xsb_source_path = minecraft_xsb.source_path;

    let resident_xwb = stage_named_asset(
        base_dir,
        &default_resident_xwb_candidates(base_dir),
        "runtime/audio/banks/resident.xwb",
    )?;
    manifest.resident_xwb_asset_path = resident_xwb.asset_path;
    manifest.resident_xwb_source_path = resident_xwb.source_path;

    let streamed_xwb = stage_named_asset(
        base_dir,
        &default_streamed_xwb_candidates(base_dir),
        "runtime/audio/banks/streamed.xwb",
    )?;
    manifest.streamed_xwb_asset_path = streamed_xwb.asset_path;
    manifest.streamed_xwb_source_path = streamed_xwb.source_path;

    let additional_xsb = stage_named_asset(
        base_dir,
        &default_additional_xsb_candidates(base_dir),
        "runtime/audio/banks/additional.xsb",
    )?;
    manifest.additional_xsb_asset_path = additional_xsb.asset_path;
    manifest.additional_xsb_source_path = additional_xsb.source_path;

    let additional_xwb = stage_named_asset(
        base_dir,
        &default_additional_xwb_candidates(base_dir),
        "runtime/audio/banks/additional.xwb",
    )?;
    manifest.additional_xwb_asset_path = additional_xwb.asset_path;
    manifest.additional_xwb_source_path = additional_xwb.source_path;

    let additional_music_xwb = stage_named_asset(
        base_dir,
        &default_additional_music_xwb_candidates(base_dir),
        "runtime/audio/banks/additional_music.xwb",
    )?;
    manifest.additional_music_xwb_asset_path = additional_music_xwb.asset_path;
    manifest.additional_music_xwb_source_path = additional_music_xwb.source_path;

    let menu_sounds_xgs = stage_named_asset(
        base_dir,
        &default_menu_sounds_xgs_candidates(base_dir),
        "runtime/audio/banks/menusounds.xgs",
    )?;
    manifest.menu_sounds_xgs_asset_path = menu_sounds_xgs.asset_path;
    manifest.menu_sounds_xgs_source_path = menu_sounds_xgs.source_path;

    let menu_sounds_xsb = stage_named_asset(
        base_dir,
        &default_menu_sounds_xsb_candidates(base_dir),
        "runtime/audio/banks/menusounds.xsb",
    )?;
    manifest.menu_sounds_xsb_asset_path = menu_sounds_xsb.asset_path;
    manifest.menu_sounds_xsb_source_path = menu_sounds_xsb.source_path;

    let menu_sounds_xwb = stage_named_asset(
        base_dir,
        &default_menu_sounds_xwb_candidates(base_dir),
        "runtime/audio/banks/menusounds.xwb",
    )?;
    manifest.menu_sounds_xwb_asset_path = menu_sounds_xwb.asset_path;
    manifest.menu_sounds_xwb_source_path = menu_sounds_xwb.source_path;

    let legacy_event_audio = stage_legacy_event_audio(base_dir, &manifest)?;
    manifest.legacy_event_audio_asset_dir = legacy_event_audio.asset_path;
    manifest.legacy_event_audio_source_dir = legacy_event_audio.source_path;

    Ok(manifest)
}

pub fn default_terrain_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_TERRAIN_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    append_local_common_candidates(&mut candidates, base_dir, &["res", "1_2_2", "terrain.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "res", "terrain.png"],
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "terrain.png"]);
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "terrain.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "TitleUpdate", "res", "terrain.png"],
    );
    append_source_client_candidates(&mut candidates, base_dir, &["Common", "res", "terrain.png"]);
    append_source_client_candidates(&mut candidates, base_dir, &["PS3", "Media", "terrain.png"]);
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("terrain.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("terrain.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("res")
            .join("terrain.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("PS3")
            .join("Media")
            .join("terrain.png"),
    );
    candidates.push(base_dir.join("assets").join("runtime").join("terrain.png"));

    dedupe_paths(candidates)
}

pub fn default_clouds_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_CLOUDS_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("environment")
            .join("clouds.png"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "environment", "clouds.png"],
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "environment", "clouds.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "environment", "clouds.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "environment", "clouds.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("environment")
            .join("clouds.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("environment")
            .join("clouds.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_gui_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_GUI_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("gui.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "gui", "gui.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "gui", "gui.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "gui", "gui.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "gui", "gui.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("gui")
            .join("gui.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("gui")
            .join("gui.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_inventory_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_INVENTORY_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("inventory.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "gui", "inventory.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "gui", "inventory.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "gui", "inventory.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "gui", "inventory.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("gui")
            .join("inventory.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("gui")
            .join("inventory.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_creative_inventory_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ALLITEMS_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("allitems.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "gui", "allitems.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "gui", "allitems.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "gui", "allitems.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "gui", "allitems.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("gui")
            .join("allitems.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("gui")
            .join("allitems.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_items_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ITEMS_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("items.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "gui", "items.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "gui", "items.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "gui", "items.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "gui", "items.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("gui")
            .join("items.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("gui")
            .join("items.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_icons_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ICONS_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("icons.png"),
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "gui", "icons.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "gui", "icons.png"],
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "gui", "icons.png"],
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "gui", "icons.png"]);
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("gui")
            .join("icons.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("gui")
            .join("icons.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_font_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_FONT_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("font")
            .join("default.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "font", "default.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "font", "default.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "font", "default.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "font", "default.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("font")
            .join("default.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("font")
            .join("default.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_mojangles_font_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MOJANGLES_TTF") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("font")
            .join("Mojangles.ttf"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "font", "Mojangles.ttf"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "font", "Mojangles.ttf"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("font")
            .join("Mojangles.ttf"),
    );

    dedupe_paths(candidates)
}

pub fn default_menu_logo_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MCLOGO_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("ui")
            .join("mclogo.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "title", "mclogo.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "title", "mclogo.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "title", "mclogo.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "title", "mclogo.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("title")
            .join("mclogo.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("title")
            .join("mclogo.png"),
    );

    dedupe_paths(candidates)
}

pub fn default_click_sound_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_CLICK_WAV") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("click.wav"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["Media", "Sound", "click.wav"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "wood click.wav"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "click.wav"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["PS3", "Media", "Sound", "click.wav"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "wood click.wav"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("click.wav"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("PS3")
            .join("Media")
            .join("Sound")
            .join("click.wav"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("wood click.wav"),
    );

    dedupe_paths(candidates)
}

pub fn default_back_sound_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_BACK_WAV") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("btn_back.wav"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "btn_Back.wav"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "btn_Back.wav"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("btn_Back.wav"),
    );

    dedupe_paths(candidates)
}

pub fn default_pop_sound_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_POP_WAV") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("pop.wav"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["Media", "Sound", "pop.wav"]);
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "pop.wav"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("pop.wav"),
    );

    dedupe_paths(candidates)
}

pub fn default_wood_click_sound_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_WOOD_CLICK_WAV") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("wood_click.wav"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "wood click.wav"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "wood click.wav"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("wood click.wav"),
    );

    dedupe_paths(candidates)
}

pub fn default_minecraft_xgs_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MINECRAFT_XGS") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("minecraft.xgs"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "audio", "Minecraft.xgs"],
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "audio", "Minecraft.xgs"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "audio", "Minecraft.xgs"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "TitleUpdate", "audio", "Minecraft.xgs"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("audio")
            .join("Minecraft.xgs"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("audio")
            .join("Minecraft.xgs"),
    );

    dedupe_paths(candidates)
}

pub fn default_minecraft_xsb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MINECRAFT_XSB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("minecraft.xsb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "audio", "minecraft.xsb"],
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "audio", "minecraft.xsb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "audio", "minecraft.xsb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "TitleUpdate", "audio", "minecraft.xsb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("audio")
            .join("minecraft.xsb"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("audio")
            .join("minecraft.xsb"),
    );

    dedupe_paths(candidates)
}

pub fn default_resident_xwb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_RESIDENT_XWB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("resident.xwb"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "audio", "resident.xwb"]);
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "audio", "resident.xwb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("audio")
            .join("resident.xwb"),
    );

    dedupe_paths(candidates)
}

pub fn default_streamed_xwb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_STREAMED_XWB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("streamed.xwb"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "audio", "streamed.xwb"]);
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "audio", "streamed.xwb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("audio")
            .join("streamed.xwb"),
    );

    dedupe_paths(candidates)
}

pub fn default_additional_xsb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ADDITIONAL_XSB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("additional.xsb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "audio", "additional.xsb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "TitleUpdate", "audio", "additional.xsb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("audio")
            .join("additional.xsb"),
    );

    dedupe_paths(candidates)
}

pub fn default_additional_xwb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ADDITIONAL_XWB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("additional.xwb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "audio", "additional.xwb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "TitleUpdate", "audio", "additional.xwb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("audio")
            .join("additional.xwb"),
    );

    dedupe_paths(candidates)
}

pub fn default_additional_music_xwb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_ADDITIONAL_MUSIC_XWB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("additional_music.xwb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "TitleUpdate", "audio", "AdditionalMusic.xwb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &[
            "Common",
            "res",
            "TitleUpdate",
            "audio",
            "AdditionalMusic.xwb",
        ],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("TitleUpdate")
            .join("audio")
            .join("AdditionalMusic.xwb"),
    );

    dedupe_paths(candidates)
}

pub fn default_menu_sounds_xgs_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MENU_SOUNDS_XGS") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("menusounds.xgs"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "Xbox", "MenuSounds.xgs"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "Xbox", "MenuSounds.xgs"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("Xbox")
            .join("MenuSounds.xgs"),
    );

    dedupe_paths(candidates)
}

pub fn default_menu_sounds_xsb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MENU_SOUNDS_XSB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("menusounds.xsb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "Xbox", "MenuSounds.xsb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "Xbox", "MenuSounds.xsb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("Xbox")
            .join("MenuSounds.xsb"),
    );

    dedupe_paths(candidates)
}

pub fn default_menu_sounds_xwb_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_MENU_SOUNDS_XWB") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("menusounds.xwb"),
    );
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["Media", "Sound", "Xbox", "MenuSounds.xwb"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "Media", "Sound", "Xbox", "MenuSounds.xwb"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("Media")
            .join("Sound")
            .join("Xbox")
            .join("MenuSounds.xwb"),
    );

    dedupe_paths(candidates)
}

pub fn default_player_skin_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("LCE_PLAYER_SKIN_PNG") {
        candidates.push(PathBuf::from(explicit));
    }

    candidates.push(
        base_dir
            .join("assets")
            .join("runtime")
            .join("mob")
            .join("char.png"),
    );
    append_local_common_candidates(&mut candidates, base_dir, &["res", "mob", "char.png"]);
    append_local_common_candidates(
        &mut candidates,
        base_dir,
        &["res", "1_2_2", "mob", "char.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "mob", "char.png"],
    );
    append_source_client_candidates(
        &mut candidates,
        base_dir,
        &["Common", "res", "1_2_2", "mob", "char.png"],
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("mob")
            .join("char.png"),
    );
    candidates.push(
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Common")
            .join("res")
            .join("1_2_2")
            .join("mob")
            .join("char.png"),
    );

    dedupe_paths(candidates)
}

pub fn stage_terrain_texture(
    base_dir: &Path,
    candidates: &[PathBuf],
) -> io::Result<RuntimeAssetManifest> {
    let mut manifest = RuntimeAssetManifest::default();
    let staged = stage_named_asset(base_dir, candidates, "runtime/terrain.png")?;
    manifest.terrain_texture_asset_path = staged.asset_path;
    manifest.terrain_texture_source_path = staged.source_path;

    Ok(manifest)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StagedAsset {
    asset_path: Option<String>,
    source_path: Option<PathBuf>,
}

fn stage_named_asset(
    base_dir: &Path,
    candidates: &[PathBuf],
    runtime_relative_path: &str,
) -> io::Result<StagedAsset> {
    stage_named_asset_with_filter(base_dir, candidates, runtime_relative_path, |_| true)
}

fn stage_click_sound_asset(base_dir: &Path, candidates: &[PathBuf]) -> io::Result<StagedAsset> {
    stage_named_wav_asset(base_dir, candidates, "runtime/audio/click.wav")
}

fn stage_named_wav_asset(
    base_dir: &Path,
    candidates: &[PathBuf],
    runtime_relative_path: &str,
) -> io::Result<StagedAsset> {
    stage_named_asset_with_filter(
        base_dir,
        candidates,
        runtime_relative_path,
        is_supported_click_sound_candidate,
    )
}

fn stage_legacy_event_audio(
    base_dir: &Path,
    manifest: &RuntimeAssetManifest,
) -> io::Result<StagedAsset> {
    let runtime_relative_dir = "runtime/audio/events";
    let output_dir = base_dir
        .join("assets")
        .join(runtime_relative_dir.replace('/', "\\"));
    std::fs::create_dir_all(&output_dir)?;

    if has_wav_files_recursive(&output_dir) {
        return Ok(StagedAsset {
            asset_path: Some(runtime_relative_dir.to_string()),
            source_path: Some(output_dir),
        });
    }

    let Some(vgmstream_cli) = find_vgmstream_cli(base_dir) else {
        return Ok(StagedAsset::default());
    };

    let banks = [
        manifest.resident_xwb_source_path.as_ref(),
        manifest.menu_sounds_xwb_source_path.as_ref(),
        manifest.additional_xwb_source_path.as_ref(),
    ];

    let mut decoded_any_bank = false;
    for bank_path in banks.into_iter().flatten() {
        if !bank_path.exists() {
            continue;
        }

        if decode_xwb_bank_events(&vgmstream_cli, bank_path, &output_dir)? {
            decoded_any_bank = true;
        }
    }

    if decoded_any_bank && has_wav_files_recursive(&output_dir) {
        return Ok(StagedAsset {
            asset_path: Some(runtime_relative_dir.to_string()),
            source_path: Some(output_dir),
        });
    }

    Ok(StagedAsset::default())
}

fn decode_xwb_bank_events(
    vgmstream_cli: &Path,
    bank_path: &Path,
    output_root: &Path,
) -> io::Result<bool> {
    let bank_name = bank_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("bank")
        .to_ascii_lowercase();
    let bank_out_dir = output_root.join(bank_name);
    std::fs::create_dir_all(&bank_out_dir)?;

    let output_pattern = bank_out_dir.join("?n__?s.wav");
    let status = Command::new(vgmstream_cli)
        .arg("-S")
        .arg("0")
        .arg("-o")
        .arg(output_pattern)
        .arg(bank_path)
        .status();

    let Ok(status) = status else {
        return Ok(false);
    };

    if !status.success() {
        return Ok(false);
    }

    Ok(has_wav_files_recursive(&bank_out_dir))
}

fn has_wav_files_recursive(directory: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_wav_files_recursive(&path) {
                return true;
            }
            continue;
        }

        let is_wav = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("wav"))
            .unwrap_or(false);
        if is_wav {
            return true;
        }
    }

    false
}

fn find_vgmstream_cli(base_dir: &Path) -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("LCE_VGMSTREAM_CLI") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Some(path);
        }
    }

    let candidates = [
        base_dir
            .join("tools")
            .join("vgmstream-win64")
            .join("vgmstream-cli.exe"),
        base_dir
            .join("..")
            .join("..")
            .join("tools")
            .join("vgmstream-win64")
            .join("vgmstream-cli.exe"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn is_supported_inventory_texture_candidate(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    if !extension.eq_ignore_ascii_case("png") {
        return false;
    }

    matches!(png_dimensions(path), Some((256, 256)))
}

fn stage_named_asset_with_filter<F>(
    base_dir: &Path,
    candidates: &[PathBuf],
    runtime_relative_path: &str,
    is_supported: F,
) -> io::Result<StagedAsset>
where
    F: Fn(&Path) -> bool,
{
    let destination_file = base_dir
        .join("assets")
        .join(runtime_relative_path.replace('/', "\\"));

    let Some(destination_parent) = destination_file.parent() else {
        return Ok(StagedAsset::default());
    };
    std::fs::create_dir_all(destination_parent)?;

    let destination_canonical = destination_file
        .canonicalize()
        .unwrap_or(destination_file.clone());

    let source = candidates
        .iter()
        .find(|path| {
            if !path.exists() || !is_supported(path) {
                return false;
            }

            let candidate_canonical = path.canonicalize().unwrap_or((*path).clone());
            candidate_canonical != destination_canonical
        })
        .cloned()
        .or_else(|| {
            candidates
                .iter()
                .find(|path| path.exists() && is_supported(path))
                .cloned()
        });
    let Some(source) = source else {
        return Ok(StagedAsset::default());
    };

    let source_canonical = source.canonicalize().unwrap_or(source.clone());

    if source_canonical != destination_canonical {
        std::fs::copy(&source, &destination_file)?;
    }

    Ok(StagedAsset {
        asset_path: Some(runtime_relative_path.to_string()),
        source_path: Some(source),
    })
}

fn is_supported_click_sound_candidate(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    if !extension.eq_ignore_ascii_case("wav") {
        return false;
    }

    is_supported_pcm_wav(path)
}

fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 24 {
        return None;
    }

    if bytes.get(0..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return None;
    }

    if bytes.get(12..16) != Some(b"IHDR") {
        return None;
    }

    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((width, height))
}

fn is_supported_pcm_wav(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };

    if bytes.len() < 44 {
        return false;
    }

    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return false;
    }

    let mut offset = 12usize;
    let mut fmt_is_supported = false;
    let mut has_non_empty_data = false;

    while offset + 8 <= bytes.len() {
        let Some(chunk_id) = bytes.get(offset..offset + 4) else {
            return false;
        };
        let Some(chunk_size) = read_u32_le(&bytes, offset + 4).map(usize::try_from) else {
            return false;
        };
        let Ok(chunk_size) = chunk_size else {
            return false;
        };

        let chunk_start = offset + 8;
        let Some(chunk_end) = chunk_start.checked_add(chunk_size) else {
            return false;
        };

        if chunk_end > bytes.len() {
            return false;
        }

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return false;
            }

            let Some(audio_format) = read_u16_le(&bytes, chunk_start) else {
                return false;
            };
            let Some(channel_count) = read_u16_le(&bytes, chunk_start + 2) else {
                return false;
            };
            let Some(sample_rate) = read_u32_le(&bytes, chunk_start + 4) else {
                return false;
            };
            let Some(bits_per_sample) = read_u16_le(&bytes, chunk_start + 14) else {
                return false;
            };

            fmt_is_supported = matches!(audio_format, 1 | 3)
                && matches!(channel_count, 1 | 2)
                && sample_rate > 0
                && matches!(bits_per_sample, 8 | 16 | 24 | 32);
        }

        if chunk_id == b"data" {
            has_non_empty_data = chunk_size > 0;
        }

        let padded_chunk_size = chunk_size + (chunk_size % 2);
        let Some(next_offset) = chunk_start.checked_add(padded_chunk_size) else {
            return false;
        };
        offset = next_offset;
    }

    fmt_is_supported && has_non_empty_data
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let data = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let data = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn append_local_common_candidates(
    candidates: &mut Vec<PathBuf>,
    base_dir: &Path,
    relative_segments: &[&str],
) {
    for common_dir in ["Common", "Commons"] {
        let mut path = base_dir.join("assets").join(common_dir);
        for segment in relative_segments {
            path.push(segment);
        }
        candidates.push(path);
    }
}

fn append_source_client_candidates(
    candidates: &mut Vec<PathBuf>,
    base_dir: &Path,
    relative_segments: &[&str],
) {
    for source_root in ["MinecraftConsoles-main", "LCEMP-main", "LCE-Original"] {
        let mut path = base_dir
            .join("..")
            .join("..")
            .join(source_root)
            .join("Minecraft.Client");
        for segment in relative_segments {
            path.push(segment);
        }
        candidates.push(path);
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    let mut seen = std::collections::BTreeSet::<OsString>::new();

    for path in paths {
        let key = path.as_os_str().to_os_string();
        if seen.insert(key) {
            deduped.push(path);
        }
    }

    deduped
}
