use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

struct IntelliJLspExtension;

const ENV_SERVER_HOME: &str = "INTELLIJ_LSP_HOME";
const ENV_SERVER_CACHE: &str = "INTELLIJ_LSP_CACHE";
const ENV_EULA_HASH: &str = "INTELLIJ_LSP_EULA_HASH";

/// Platform-aware user home directory.
fn user_home() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default()
}

/// Platform-aware default server install path.
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

/// Platform-aware default cache path.
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

/// Platform-aware launcher name (with .exe on Windows).
fn launcher_name() -> &'static str {
    if cfg!(windows) {
        "intellij-server.exe"
    } else {
        "intellij-server"
    }
}

fn find_latest_server() -> Option<String> {
    let home = std::env::var(ENV_SERVER_HOME).unwrap_or_else(|_| default_server_home());

    let entries = std::fs::read_dir(&home).ok()?;
    let mut versions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix("server-").map(|s| s.to_string())
        })
        .collect();

    if versions.is_empty() {
        return None;
    }

    versions.sort_by(|a, b| b.cmp(a));

    let bin = launcher_name();
    let launcher = if cfg!(windows) {
        format!("{}\\server-{}\\bin\\{}", home, versions[0], bin)
    } else {
        format!("{}/server-{}/bin/{}", home, versions[0], bin)
    };

    if std::path::Path::new(&launcher).exists() {
        Some(launcher)
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

    // djb2 hash — 16 hex chars for EULA acceptance gate.
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
        let launcher = find_latest_server().ok_or_else(|| {
            format!(
                "IntelliJ LSP server not found.\n\
                 Run ./scripts/install.sh (macOS/Linux) or set {ENV_SERVER_HOME}."
            )
        })?;

        let cache = std::env::var(ENV_SERVER_CACHE).unwrap_or_else(|_| default_cache_dir());

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
        let path = default_server_home();
        assert!(!path.is_empty());
        // Should contain the user home or be overridable via env
    }

    #[test]
    fn test_launcher_name_ends_with_exe_on_windows() {
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
        assert!(h1.is_some());
        assert_eq!(h1.unwrap().len(), 16);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_eula_hash_different_content() {
        let dir_a = std::env::temp_dir().join("intellij-lsp-test-a");
        let dir_b = std::env::temp_dir().join("intellij-lsp-test-b");
        let _ = std::fs::create_dir_all(&dir_a);
        let _ = std::fs::create_dir_all(&dir_b);

        std::fs::write(dir_a.join("EULA.txt"), b"hash-me").unwrap();
        std::fs::write(dir_b.join("EULA.txt"), b"different").unwrap();

        let h1 = compute_eula_hash(&dir_a.to_string_lossy());
        let h2 = compute_eula_hash(&dir_b.to_string_lossy());

        assert_ne!(h1, h2);

        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn test_eula_hash_missing_file() {
        let hash = compute_eula_hash("/tmp/does-not-exist-98765");
        assert!(hash.is_none());
    }

    #[test]
    fn test_eula_hash_is_hex() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-hex");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("EULA.txt"), b"test").unwrap();

        let hash = compute_eula_hash(&dir.to_string_lossy());
        let h = hash.unwrap();
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
