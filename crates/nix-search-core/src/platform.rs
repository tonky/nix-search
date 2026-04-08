pub fn detect_current_platform() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let nix_os = match os {
        "macos" => "darwin",
        other => other,
    };
    format!("{}-{}", arch, nix_os)
}
