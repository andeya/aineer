fn main() {
    // Embed icon into .exe on Windows
    #[cfg(target_os = "windows")]
    {
        let rc_path = std::path::Path::new("src/resources.rc");
        if rc_path.exists() {
            if let Ok(_) = std::process::Command::new("windres")
                .arg("--version")
                .output()
            {
                let status = std::process::Command::new("windres")
                    .args(["src/resources.rc", "-o"])
                    .arg(format!("{}/resources.o", std::env::var("OUT_DIR").unwrap()))
                    .status()
                    .expect("failed to run windres");
                if status.success() {
                    println!(
                        "cargo:rustc-link-arg={}",
                        format!("{}/resources.o", std::env::var("OUT_DIR").unwrap())
                    );
                }
            } else {
                println!("cargo:warning=windres not found — .exe will not have an icon");
            }
        }
    }
}
