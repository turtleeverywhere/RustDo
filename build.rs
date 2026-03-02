use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only add Windows executable resources for Windows targets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"),
    );
    let png_icon = manifest_dir.join("Rust_do_icon.png");
    if !png_icon.exists() {
        panic!("Icon file not found: {}", png_icon.display());
    }

    let out_dir =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is not set by Cargo build"));
    let ico_icon = out_dir.join("app_icon.ico");
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    create_multi_size_ico(&png_icon, &ico_icon);
    println!("cargo:rerun-if-changed=Rust_do_icon.png");

    if target_env == "gnu" {
        embed_icon_with_windres(&ico_icon, &out_dir);
    } else {
        winres::WindowsResource::new()
            .set_icon(
                ico_icon
                    .to_str()
                    .expect("ICO path contains invalid UTF-8 characters"),
            )
            .compile()
            .expect("Failed to compile Windows resources");
    }
}

fn create_multi_size_ico(png_icon: &PathBuf, ico_icon: &PathBuf) {
    let source = image::open(png_icon)
        .expect("Failed to open Rust_do_icon.png")
        .into_rgba8();
    let sizes: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in sizes {
        let resized = image::imageops::resize(
            &source,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        let icon_entry = ico::IconDirEntry::encode(&icon_image)
            .expect("Failed to encode ICO entry from RGBA data");
        icon_dir.add_entry(icon_entry);
    }

    let mut file = std::fs::File::create(ico_icon).expect("Failed to create app_icon.ico");
    icon_dir
        .write(&mut file)
        .expect("Failed to write multi-size ICO file");
}

fn embed_icon_with_windres(ico_icon: &PathBuf, out_dir: &PathBuf) {
    let rc_path = out_dir.join("app_icon.rc");
    let obj_path = out_dir.join("app_icon.o");
    let icon_path = ico_icon
        .to_string_lossy()
        .replace('\\', "/");
    let rc_content = format!("1 ICON \"{icon_path}\"\n");
    std::fs::write(&rc_path, rc_content).expect("Failed to write app_icon.rc");

    let mut attempted = Vec::new();
    let candidates = ["windres", "x86_64-w64-mingw32-windres"];
    let mut compiled = false;

    for candidate in candidates {
        attempted.push(candidate);
        let status = Command::new(candidate)
            .arg(&rc_path)
            .arg("-O")
            .arg("coff")
            .arg("-o")
            .arg(&obj_path)
            .status();

        if let Ok(exit_status) = status {
            if exit_status.success() {
                compiled = true;
                break;
            }
        }
    }

    if !compiled {
        panic!("Failed to run windres. Tried: {}", attempted.join(", "));
    }

    println!(
        "cargo:rustc-link-arg={}",
        obj_path
            .to_str()
            .expect("Resource object path contains invalid UTF-8")
    );
}
