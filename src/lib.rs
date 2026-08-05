use sha2::{Digest, Sha256};
use std::fs;
use zed_extension_api::{
    self as zed, download_file, make_file_executable, set_language_server_installation_status,
    Command, DownloadedFileType, Extension, LanguageServerId, LanguageServerInstallationStatus,
    Result, Worktree,
};

struct IntelliJLspExtension {
    cached_binary_path: Option<String>,
}

fn find_binary_in(dir: &str, depth: u32) -> Option<String> {
    if depth > 4 {
        return None;
    }
    for entry in fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_binary_in(&path.to_string_lossy(), depth + 1) {
                return Some(found);
            }
        } else if entry.file_name() == "intellij-server" {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

impl IntelliJLspExtension {
    fn server_version_dir(version: &str) -> String {
        format!("intellij-server-{}", version)
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        // Check if any version is already installed in the sandbox.
        if let Ok(entries) = fs::read_dir(".") {
            let mut latest: Option<(String, String)> = None;
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(version) = name.strip_prefix("intellij-server-") {
                    // Try to find the binary at known paths.
                    let candidates = [format!("{}/bin/intellij-server", name)];
                    for c in &candidates {
                        if fs::metadata(c).is_ok_and(|s| s.is_file()) {
                            match &latest {
                                Some((v, _)) if version > v.as_str() => {
                                    latest = Some((version.to_string(), c.clone()));
                                }
                                None => {
                                    latest = Some((version.to_string(), c.clone()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if let Some((_, path)) = latest {
                self.cached_binary_path = Some(path.clone());
                return Ok(path);
            }
        }

        // Not installed — download.
        set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );

        // Fetch latest server info from Open VSX → VSIX → server-bundle.json.
        download_file(
            "https://open-vsx.org/api/JetBrains/intellij-server/latest",
            "vsix-meta.json",
            DownloadedFileType::Uncompressed,
        )
        .map_err(|e| format!("failed to fetch metadata: {e}"))?;
        let body = fs::read_to_string("vsix-meta.json").map_err(|e| format!("read error: {e}"))?;
        fs::remove_file("vsix-meta.json").ok();

        let meta: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("invalid JSON: {e}"))?;
        let vsix_url = meta
            .get("files")
            .and_then(|f| f.get("download"))
            .and_then(|v| v.as_str())
            .or_else(|| meta.get("download").and_then(|v| v.as_str()))
            .ok_or("missing VSIX download URL")?;

        download_file(vsix_url, "vsix", DownloadedFileType::Zip)
            .map_err(|e| format!("VSIX download failed: {e}"))?;
        let bundle = fs::read_to_string("vsix/extension/server-bundle.json")
            .map_err(|e| format!("read bundle: {e}"))?;
        fs::remove_dir_all("vsix").ok();

        let bundle_json: serde_json::Value =
            serde_json::from_str(&bundle).map_err(|e| format!("invalid bundle: {e}"))?;
        let server_url = bundle_json
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("missing server URL in bundle")?;
        let version = bundle_json
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or("missing version in bundle")?;

        let version_dir = Self::server_version_dir(version);

        if fs::metadata(&version_dir).is_err() {
            set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Downloading,
            );

            download_file(server_url, &version_dir, DownloadedFileType::Zip)
                .map_err(|e| format!("server download failed: {e}"))?;
        }

        let binary =
            find_binary_in(&version_dir, 0).ok_or("server binary not found after extraction")?;

        make_file_executable(&binary).map_err(|e| format!("chmod failed: {e}"))?;
        self.cached_binary_path = Some(binary.clone());
        Ok(binary)
    }
}

impl Extension for IntelliJLspExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Command> {
        let binary_path = self.language_server_binary_path(language_server_id)?;
        Ok(Command {
            command: binary_path,
            args: vec!["--stdio".to_string()],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Read EULA.txt and compute acceptance hash.
        let eula_path = if let Some(ref path) = self.cached_binary_path {
            // EULA.txt is in the server root dir (parent of bin/)
            let bin_dir = std::path::Path::new(path).parent().unwrap();
            bin_dir.parent().unwrap().join("EULA.txt")
        } else {
            // Fallback: search for EULA.txt
            let mut found = None;
            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let eula = path.join("EULA.txt");
                        if eula.exists() {
                            found = Some(eula);
                            break;
                        }
                    }
                }
            }
            found.unwrap_or_else(|| std::path::PathBuf::from("EULA.txt"))
        };

        let data = fs::read(&eula_path).ok();
        let hash = data.map(|d| {
            let mut hasher = Sha256::new();
            hasher.update(&d);
            // First 16 hex chars (64 bits) of SHA-256
            format!(
                "{:016x}",
                u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap())
            )
        });

        Ok(Some(serde_json::json!({
            "eulaHash": hash
        })))
    }
}

zed::register_extension!(IntelliJLspExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_binary_in_empty_dir() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-empty");
        let _ = fs::create_dir_all(&dir);
        assert!(find_binary_in(&dir.to_string_lossy(), 0).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_binary_in_nested() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-nested");
        let bin_dir = dir.join("nested").join("bin");
        let _ = fs::create_dir_all(&bin_dir);
        fs::write(bin_dir.join("intellij-server"), b"fake").unwrap();
        let found = find_binary_in(&dir.to_string_lossy(), 0);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with("intellij-server"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_server_version_dir() {
        assert_eq!(
            IntelliJLspExtension::server_version_dir("1.2.3"),
            "intellij-server-1.2.3"
        );
    }

    #[test]
    fn test_eula_hash_sha256() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-eula");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("EULA.txt"), b"ACCEPT_ME").unwrap();
        let data = fs::read(dir.join("EULA.txt")).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!(
            "{:016x}",
            u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap())
        );
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_eula_hash_deterministic() {
        let dir = std::env::temp_dir().join("intellij-lsp-test-det");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("EULA.txt"), b"same content").unwrap();

        let data = fs::read(dir.join("EULA.txt")).unwrap();
        let h1 = {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!(
                "{:016x}",
                u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap())
            )
        };
        let h2 = {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!(
                "{:016x}",
                u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap())
            )
        };
        assert_eq!(h1, h2);
        let _ = fs::remove_dir_all(&dir);
    }
}
