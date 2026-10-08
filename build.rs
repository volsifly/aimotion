fn main() {
    // xkbcommon needs no C headers here. If a minimal desktop has only versioned
    // runtime libraries, provide linker names in Cargo's build directory.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") { return; }
    let directory = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("native-link");
    std::fs::create_dir_all(&directory).unwrap();
    for library in ["xkbcommon", "xkbcommon-x11"] {
        let name = format!("lib{library}.so.0");
        let output = std::process::Command::new("cc").arg(format!("-print-file-name={name}")).output().expect("C compiler is required");
        let source = std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if source.is_file() {
            let link = directory.join(format!("lib{library}.so"));
            if !link.exists() {
                #[cfg(unix)]
                std::os::unix::fs::symlink(source, link).unwrap();
            }
        }
    }
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rerun-if-changed=build.rs");
}
