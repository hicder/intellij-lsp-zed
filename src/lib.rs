use zed_extension_api::{
    self as zed, download_file, make_file_executable, Command, DownloadedFileType, Extension,
    LanguageServerId, Result, Worktree,
};

struct IntelliJLspExtension;

const ENV_SERVER_HOME: &str = "INTELLIJ_LSP_HOME";
const ENV_SERVER_CACHE: &str = "INTELLIJ_LSP_CACHE";
const ENV_EULA_HASH: &str = "INTELLIJ_LSP_EULA_HASH";

const OPEN_VSX_API: &str = "https://open-vsx.org/api/JetBrains/intellij-server/latest";

fn user_home() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default()
}

fn default_server_home() -> String {
    let home = user_home();
    if cfg!(windows) {
        format!("{}\\AppData\\Local\\intellij-lsp", home)
    } else if cfg!(target_os = "macos") {
        format!("{}/Library/Application Support/intellij-lsp", home)
    } else {
        format!("{}/.local/share/intellij-lsp", home)
    }
}

fn default_cache_dir() -> String {
    let home = user_home();
    if cfg!(windows) {
        format!("{}\\AppData\\Local\\intellij-lsp-zed", home)
    } else if cfg!(target_os = "macos") {
        format!("{}/Library/Caches/intellij-lsp-zed", home)
    } else {
        format!("{}/.cache/intellij-lsp-zed", home)
    }
}

fn launcher_name() -> &'static str {
    if cfg!(windows) {
        "intellij-server.exe"
    } else {
        "intellij-server"
    }
}

fn launcher_path(home: &str, version: &str) -> String {
    if cfg!(windows) {
        format!("{}\\server-{}\\bin\\{}", home, version, launcher_name())
    } else {
        format!("{}/server-{}/bin/{}", home, version, launcher_name())
    }
}

/// Fetch the latest server bundle download URL and version from Open VSX.
fn fetch_server_info() -> Option<(String, String)> {
    // Download the VSIX metadata JSON from Open VSX API.
    let tmp = std::env::temp_dir();
    let meta_path = tmp.join("intellij-vsix-meta.json");
    download_file(
        OPEN_VSX_API,
        &meta_path.to_string_lossy(),
        DownloadedFileType::Uncompressed,
    )
    .ok()?;
    let body = std::fs::read_to_string(&meta_path).ok()?;
    std::fs::remove_file(&meta_path).ok();

    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let download_url = json
        .get("files")
        .and_then(|f| f.get("download"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("download").and_then(|v| v.as_str()))?;

    // Download and extract the VSIX to get server-bundle.json
    let vsix_path = tmp.join("intellij-server.vsix");
    download_file(
        download_url,
        &vsix_path.to_string_lossy(),
        DownloadedFileType::Zip,
    )
    .ok()?;

    let extracted = vsix_path.with_extension("");
    let bundle_path = extracted.join("extension").join("server-bundle.json");
    let bundle = std::fs::read_to_string(&bundle_path).ok()?;
    let bundle_json: serde_json::Value = serde_json::from_str(&bundle).ok()?;

    let url = bundle_json.get("url")?.as_str()?.to_string();
    let version = bundle_json.get("version")?.as_str()?.to_string();

    std::fs::remove_dir_all(extracted).ok();

    Some((url, version))
}

/// Ensure the server is installed, downloading it if necessary.
fn ensure_server_installed() -> Option<String> {
    let home = std::env::var(ENV_SERVER_HOME).unwrap_or_else(|_| default_server_home());

    // Check if any version is already installed.
    if let Ok(entries) = std::fs::read_dir(&home) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(version) = name.strip_prefix("server-") {
                    let lp = launcher_path(&home, version);
                    if std::path::Path::new(&lp).exists() {
                        return Some(lp);
                    }
                }
            }
        }
    }

    // Not installed — download and extract the server bundle.
    let (url, version) = fetch_server_info()?;
    let _ = std::fs::create_dir_all(&home);

    let server_dir = format!("{}/server-{}", home, version);
    let archive_path = format!("{}/server-{}.zip", home, version);

    // download_file with Zip type automatically extracts the archive.
    download_file(&url, &archive_path, DownloadedFileType::Zip).ok()?;

    // The zip contains a top-level dir like "intellij-server-263.2689.0/".
    let extracted = archive_path.trim_end_matches(".zip");
    if std::path::Path::new(extracted).is_dir() && extracted != server_dir {
        if let Ok(entries) = std::fs::read_dir(extracted) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    std::fs::rename(entry.path(), &server_dir).ok();
                    break;
                }
            }
        }
        std::fs::remove_dir_all(extracted).ok();
    }

    std::fs::remove_file(&archive_path).ok();

    let lp = launcher_path(&home, &version);
    make_file_executable(&lp).ok();

    if std::path::Path::new(&lp).exists() {
        Some(lp)
    } else {
        None
    }
}

fn compute_eula_hash(server_dir: &str) -> Option<String> {
    let eula_path = if cfg!(windows) {
        format!("{}\\EULA.txt", server_dir)
    } else {
        format!("{}/EULA.txt", server_dir)
    };
    let data = std::fs::read(&eula_path).ok()?;

    let mut hash: u64 = 5381;
    for &byte in &data {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    Some(format!("{:016x}", hash))
}

impl Extension for IntelliJLspExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Command> {
        let launcher = ensure_server_installed().ok_or_else(|| {
            "IntelliJ LSP server could not be installed. Check your internet connection."
                .to_string()
        })?;

        let cache = std::env::var(ENV_SERVER_CACHE).unwrap_or_else(|_| default_cache_dir());
        let _ = std::fs::create_dir_all(&cache);

        Ok(Command {
            command: launcher,
            args: vec!["--stdio".to_string(), "--system-path".to_string(), cache],
            env: vec![],
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let eula_hash = std::env::var(ENV_EULA_HASH).ok().or_else(|| {
            let home = std::env::var(ENV_SERVER_HOME).unwrap_or_else(|_| default_server_home());
            let entries = std::fs::read_dir(&home).ok()?;
            let dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_prefix("server-")
                        .map(|v| (v.to_string(), e.path().to_string_lossy().to_string()))
                })
                .collect();

            let server_dir = dirs
                .into_iter()
                .max_by_key(|(v, _)| v.clone())
                .map(|(_, d)| d)?;

            compute_eula_hash(&server_dir)
        });

        Ok(Some(serde_json::json!({
            "eulaHash": eula_hash
        })))
    }
}

zed::register_extension!(IntelliJLspExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_home_is_absolute() {
        assert!(!default_server_home().is_empty());
    }

    #[test]
    fn test_launcher_name() {
        let name = launcher_name();
        if cfg!(windows) {
            assert!(name.ends_with(".exe"));
        } else {
            assert!(!name.ends_with(".exe"));
        }
    }

    #[test]
    fn test_compute_eula_hash_deterministic() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-1");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("EULA.txt"), b"ACCEPT_ME").unwrap();

        let h1 = compute_eula_hash(&dir.to_string_lossy());
        let h2 = compute_eula_hash(&dir.to_string_lossy());
        assert_eq!(h1, h2);
        assert_eq!(h1.unwrap().len(), 16);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_eula_hash_different_content() {
        let dir_a = std::env::temp_dir().join("lsp-test-a");
        let dir_b = std::env::temp_dir().join("lsp-test-b");
        let _ = std::fs::create_dir_all(&dir_a);
        let _ = std::fs::create_dir_all(&dir_b);
        std::fs::write(dir_a.join("EULA.txt"), b"hash-me").unwrap();
        std::fs::write(dir_b.join("EULA.txt"), b"different").unwrap();
        assert_ne!(
            compute_eula_hash(&dir_a.to_string_lossy()),
            compute_eula_hash(&dir_b.to_string_lossy())
        );
        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn test_eula_hash_missing_file() {
        assert!(compute_eula_hash("/tmp/does-not-exist-98765").is_none());
    }

    #[test]
    fn test_eula_hash_is_hex() {
        let dir = std::env::temp_dir().join("lsp-test-hex");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("EULA.txt"), b"test").unwrap();
        let h = compute_eula_hash(&dir.to_string_lossy()).unwrap();
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
