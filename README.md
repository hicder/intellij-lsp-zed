# IntelliJ LSP for Zed

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Zed extension that brings [IntelliJ IDEA's LSP server][1] for Java & Kotlin to
the [Zed editor][2] — code completion, navigation, refactorings, inspections,
and quick-fixes for Maven, Gradle, and Bazel projects.

**Zero setup required.** The extension automatically downloads and installs the
server on first launch.

[1]: https://blog.jetbrains.com/idea/2026/08/intellij-idea-goes-lsp/
[2]: https://zed.dev

## Quick Start

1. Install the extension from Zed's extension browser
   (`Cmd+Shift+P` → `zed: extensions` → search "IntelliJ LSP")
2. Open a Java or Kotlin project
3. The server downloads and starts automatically on first launch (~368 MB,
   one-time)

That's it. No scripts, no manual configuration.

## Zed Settings

Add to `~/.config/zed/settings.json` to use the IntelliJ server instead of
Zed's defaults:

```json
"languages": {
  "Java": {
    "language_servers": ["intellij-server", "!jdtls"]
  },
  "Kotlin": {
    "language_servers": ["intellij-server", "!kotlin-language-server"]
  }
}
```

The `!` prefix disables Zed's built-in servers to avoid duplicate diagnostics.

## How It Works

1. The extension checks if the IntelliJ LSP server is installed
2. If not, it downloads the latest version from JetBrains CDN (~368 MB)
3. The archive is extracted to a platform-specific directory
4. The server reads the bundled `EULA.txt` and computes the acceptance hash
5. Your project is imported (Maven/Gradle/Bazel) and language features activate

## Platform Defaults

|                  | macOS                                                     | Linux                                      | Windows                                    |
| ---------------- | --------------------------------------------------------- | ------------------------------------------ | ------------------------------------------ |
| Server install   | `~/Library/Application Support/intellij-lsp`              | `~/.local/share/intellij-lsp`              | `%LOCALAPPDATA%\intellij-lsp`              |
| Cache / logs     | `~/Library/Caches/intellij-lsp-zed`                       | `~/.cache/intellij-lsp-zed`                | `%LOCALAPPDATA%\intellij-lsp-zed`          |
| Extension folder | `~/Library/Application Support/Zed/extensions/installed/` | `~/.local/share/zed/extensions/installed/` | `%LOCALAPPDATA%\Zed\extensions\installed\` |

## Environment Variables

| Variable                 | Description                       |
| ------------------------ | --------------------------------- |
| `INTELLIJ_LSP_HOME`      | Override server install directory |
| `INTELLIJ_LSP_CACHE`     | Override server cache / logs      |
| `INTELLIJ_LSP_EULA_HASH` | Skip auto-detection of EULA hash  |

## Development

```sh
# Build the extension
cargo build --release --target wasm32-wasip2

# Run tests
cargo test

# Install locally
cp target/wasm32-wasip2/release/intellij_lsp_zed.wasm extension.wasm
cp extension.wasm extension.toml \
  ~/Library/Application\ Support/Zed/extensions/installed/intellij-lsp-zed/
```

## Requirements

- **macOS**, **Linux**, or **Windows**
- **Zed** editor (any recent version)
- Internet connection on first launch (to download the server)

## Caveats

- **Evaluation period**: preview builds are valid for 30 days. The extension
  auto-updates when new builds are released.
- **License**: during the preview the extension is free. After the preview a
  paid IntelliJ IDEA Ultimate subscription will be required.
- **Library sources**: `Cmd+Click` into JDK/Spring classes won't open their
  source (Zed doesn't support `jar://` URIs yet). Your own code navigation
  works fine.

## License

[MIT](LICENSE) — the extension is MIT-licensed.

The IntelliJ LSP server itself is proprietary software by JetBrains, subject
to its own [EULA](https://www.jetbrains.com/legal/).
