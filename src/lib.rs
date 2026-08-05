use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

struct IntelliJLspExtension;

const ENV_SERVER_HOME: &str = "INTELLIJ_LSP_HOME";
const ENV_SERVER_CACHE: &str = "INTELLIJ_LSP_CACHE";
const ENV_EULA_HASH: &str = "INTELLIJ_LSP_EULA_HASH";

fn find_latest_server() -> Option<String> {
    let home = std::env::var(ENV_SERVER_HOME).unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}/.local/share/intellij-lsp", home)
    });

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
    let launcher = format!("{}/server-{}/bin/intellij-server", home, versions[0]);

    if std::path::Path::new(&launcher).exists() {
        Some(launcher)
    } else {
        None
    }
}

fn compute_eula_hash(server_dir: &str) -> Option<String> {
    let eula_path = format!("{}/EULA.txt", server_dir);
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
                 Run ./scripts/install.sh or set {ENV_SERVER_HOME}."
            )
        })?;

        let cache = std::env::var(ENV_SERVER_CACHE).unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{}/.cache/intellij-lsp-zed", home)
        });

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
            let home = std::env::var(ENV_SERVER_HOME).unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("{}/.local/share/intellij-lsp", home)
            });
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
