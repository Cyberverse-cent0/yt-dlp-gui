fn main() {
    #[cfg(feature = "prebuilt-ytdlp")]
    {
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-env-changed=PREBUILT_YTDLP_PATH");

        // If a custom path is provided via environment variable, use it
        if let Ok(path) = std::env::var("PREBUILT_YTDLP_PATH") {
            println!("cargo:rustc-env=PREBUILT_YTDLP_PATH={}", path);
            return;
        }

        // Otherwise, download yt-dlp to the target directory
        use std::env;
        use std::fs;
        use std::path::PathBuf;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let ytdlp_name = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else if cfg!(target_os = "macos") {
            "yt-dlp_macos"
        } else {
            "yt-dlp_linux"
        };
        let ytdlp_path = out_dir.join(ytdlp_name);

        if !ytdlp_path.exists() {
            println!("Downloading prebuilt yt-dlp to {:?}", ytdlp_path);

            let url = if cfg!(target_os = "windows") {
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
            } else if cfg!(target_os = "macos") {
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
            } else {
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
            };

            let response = reqwest::blocking::get(url).expect("Failed to download yt-dlp");
            let bytes = response.bytes().expect("Failed to read yt-dlp bytes");
            fs::write(&ytdlp_path, bytes).expect("Failed to write yt-dlp");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&ytdlp_path).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&ytdlp_path, perms).expect("Failed to set yt-dlp permissions");
            }
        }

        println!("cargo:rustc-env=PREBUILT_YTDLP_PATH={}", ytdlp_path.display());
    }
}
