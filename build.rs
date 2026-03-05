use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(miles_audio_link)");

    let miles_feature_enabled = std::env::var_os("CARGO_FEATURE_MILES_AUDIO").is_some();
    if !miles_feature_enabled || !cfg!(target_os = "windows") {
        return;
    }

    let crate_root = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo should provide CARGO_MANIFEST_DIR"),
    );

    let candidates = [
        crate_root
            .join("assets")
            .join("Common")
            .join("Windows64")
            .join("Miles")
            .join("lib"),
        crate_root
            .join("assets")
            .join("Commons")
            .join("Windows64")
            .join("Miles")
            .join("lib"),
        crate_root
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Windows64")
            .join("Miles")
            .join("lib"),
        crate_root
            .join("..")
            .join("..")
            .join("LCEMP-main")
            .join("Minecraft.Client")
            .join("Windows64")
            .join("Miles")
            .join("lib"),
    ];

    let Some(link_path) = candidates.into_iter().find(|path| path.exists()) else {
        println!(
            "cargo:warning=miles_audio enabled but Miles library path not found; disabling Miles link backend"
        );
        return;
    };

    println!("cargo:rustc-link-search=native={}", link_path.display());
    println!("cargo:rustc-link-lib=mss64");
    println!("cargo:rustc-link-arg-bin=miles_smoke=mss64.lib");
    println!("cargo:rustc-link-arg-bin=bevy_client=mss64.lib");
    println!("cargo:rustc-cfg=miles_audio_link");
}
