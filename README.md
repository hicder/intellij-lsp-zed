# IntelliJ LSP for Zed

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Zed extension that brings [IntelliJ IDEA's LSP server][1] for Java & Kotlin to
the [Zed editor][2] — code completion, navigation, refactorings, inspections,
and quick-fixes for Maven, Gradle, and Bazel projects.

[1]: https://blog.jetbrains.com/idea/2026/08/intellij-idea-goes-lsp/
[2]: https://zed.dev

## Quick Start

### 1. Download the IntelliJ LSP server

```sh
./scripts/install.sh
```

This downloads the server (~368 MB) to `~/.local/share/intellij-lsp/` and
writes `intellij-lsp-zed.md` with setup instructions.

### 2. Build and install the extension

```sh
cargo build --release --target wasm32-wasip2
cp target/wasm32-wasip2/release/intellij_lsp_zed.wasm extension.wasm
cp extension.wasm extension.toml \
  ~/Library/Application\ Support/Zed/extensions/installed/intellij-lsp-zed/
```

On Linux the extensions folder is `~/.local/share/zed/extensions/installed/`.

### 3. Configure Zed

In `~/.config/zed/settings.json`, add to the `"languages"` section:

```json
"Java": {
  "language_servers": ["intellij-server", "!jdtls"]
},
"Kotlin": {
  "language_servers": ["intellij-server", "!kotlin-language-server"]
}
```

The `!` prefix disables Zed's default servers to avoid duplicate diagnostics.

Restart Zed (`Cmd+Shift+P` → `zed: reload`) and open a Java or Kotlin project.

## How It Works

1. The extension auto-discovers the latest server version at
   `~/.local/share/intellij-lsp/server-<version>/`.
2. On first launch, it reads the bundled `EULA.txt` and computes the acceptance
   hash automatically.
3. The server index is stored at `~/.cache/intellij-lsp-zed/`.

## Environment Variables

| Variable                 | Default                       | Description              |
| ------------------------ | ----------------------------- | ------------------------ |
| `INTELLIJ_LSP_HOME`      | `~/.local/share/intellij-lsp` | Server install directory |
| `INTELLIJ_LSP_CACHE`     | `~/.cache/intellij-lsp-zed`   | Server cache / logs      |
| `INTELLIJ_LSP_EULA_HASH` | (computed from EULA.txt)      | Skip auto-detection      |

## Requirements

- **macOS** or **Linux**
- **Zed** editor (any recent version)
- **Rust** (only to build the extension Wasm)
- ~370 MB free disk space for the server bundle

## Caveats

- **Evaluation period**: preview builds are valid for 30 days. JetBrains ships
  new builds every 2 weeks — run `./scripts/install.sh` periodically.
- **License**: during the preview the extension is free. After the preview a
  paid IntelliJ IDEA Ultimate subscription will be required.
- **Library sources**: `Cmd+Click` into JDK/Spring classes won't open their
  source (Zed doesn't support `jar://` URIs yet). Your own code navigation
  works fine.

## License

[MIT](LICENSE) — the extension and install script are MIT-licensed.

The IntelliJ LSP server itself is proprietary software by JetBrains, subject
to its own [EULA](https://www.jetbrains.com/legal/).
