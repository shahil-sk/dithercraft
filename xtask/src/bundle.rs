//! `cargo xtask bundle`: build the release app and wrap it as `dist/DitherCraft.app` (macOS).

use std::path::Path;
use std::process::Command;

pub fn run(root: &Path) -> Result<(), String> {
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(root)
        .args(["build", "--release", "-p", "dithercraft", "-p", "dithercraft-cli"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("release build failed".into());
    }
    let target = std::env::var("CARGO_TARGET_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| root.join("target"));
    let app = root.join("dist/DitherCraft.app/Contents");
    let _ = std::fs::remove_dir_all(root.join("dist/DitherCraft.app"));
    std::fs::create_dir_all(app.join("MacOS")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(app.join("Resources")).map_err(|e| e.to_string())?;
    std::fs::copy(target.join("release/dithercraft"), app.join("MacOS/DitherCraft")).map_err(|e| format!("copy app: {e}"))?;
    std::fs::copy(target.join("release/dithercraft-cli"), app.join("MacOS/dithercraft-cli")).map_err(|e| format!("copy cli: {e}"))?;
    if root.join("assets/app-icon/dithercraft.icns").exists() {
        let _ = std::fs::copy(root.join("assets/app-icon/dithercraft.icns"), app.join("Resources/DitherCraft.icns"));
    }
    let sha = std::env::var("DITHERCRAFT_BUILD_SHA").unwrap_or_else(|_| "unknown".into());
    let info = info_plist(env!("CARGO_PKG_VERSION"), &sha);
    std::fs::write(app.join("Info.plist"), info).map_err(|e| e.to_string())?;
    println!("built {}", root.join("dist/DitherCraft.app").display());
    Ok(())
}

fn info_plist(version: &str, sha: &str) -> String {
    let short = version.split('-').next().unwrap_or(version);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>DitherCraft</string>
    <key>CFBundleDisplayName</key>
    <string>DitherCraft</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.dithercraft</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{short}</string>
    <key>CFBundleExecutable</key>
    <string>DitherCraft</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>DitherCraftBuildSha</key>
    <string>{sha}</string>
</dict>
</plist>
"#
    )
}
