# IntelliJ LSP for Zed

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Zed extension that brings [IntelliJ IDEA's LSP server][1] for Java & Kotlin to
the [Zed editor][2] — code completion, navigation, refactorings, inspections,
and quick-fixes for Maven, Gradle, and Bazel projects.

**Zero setup.** The extension downloads and installs the server automatically
on first launch.

[1]: https://blog.jetbrains.com/idea/2026/08/intellij-idea-goes-lsp/
[2]: https://zed.dev

## Install

1. Open Zed → `Cmd+Shift+P` → `zed: extensions` → search **IntelliJ LSP**
2. Open a Java or Kotlin project
3. The server (~368 MB) downloads and starts automatically — one-time, first
   launch only

That's it.

## Disable Zed's Built-in Java/Kotlin Servers (optional)

The extension registers the IntelliJ server for Java and Kotlin automatically.
Zed also ships its own servers (`jdtls`, `kotlin-language-server`), so you'll
get duplicate diagnostics unless you disable them. Add to
`~/.config/zed/settings.json`:

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

The `!` prefix disables the built-in server for that language.

## How It Works

1. The extension checks its cache for an existing server
2. If none is found, it fetches the latest version info from
   [Open VSX](https://open-vsx.org/extension/JetBrains/intellij-server)
3. It downloads the server bundle from JetBrains' CDN and extracts it
4. The EULA acceptance hash is computed from the bundled `EULA.txt`
5. Your project imports (Maven/Gradle/Bazel) and language features activate

Fresh installs always get the latest published build. Cached versions are
reused on subsequent launches.

## Evaluation & License

- During the preview the extension is **free** — each build is valid for
  **30 days** from its release date
- After the preview, an IntelliJ IDEA Ultimate subscription will be required
- If the server stops working after ~30 days, clear the extension's cache to
  fetch a newer build (see Troubleshooting)

## Troubleshooting

| Problem                                 | Fix                                                                                                                                                                                                                  |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Server won't start / evaluation expired | Clear the server cache: `rm -rf ~/Library/Application\ Support/Zed/extensions/work/intellij-lsp-zed` (Linux: `~/.local/share/zed/extensions/work/`, Windows: `%LOCALAPPDATA%\Zed\extensions\work\`), then reload Zed |
| Download fails                          | Check your internet connection, then retry — the extension resumes cleanly                                                                                                                                           |
| Duplicate diagnostics                   | Add the `language_servers` config above to disable Zed's built-ins                                                                                                                                                   |

## Development

```sh
# Build the extension (requires Rust + wasm32-wasip2 target)
cargo build --release --target wasm32-wasip2

# Run tests
cargo test

# Install locally (macOS; adjust path on Linux/Windows)
cp target/wasm32-wasip2/release/intellij_lsp_zed.wasm extension.wasm
cp extension.wasm extension.toml \
  ~/Library/Application\ Support/Zed/extensions/installed/intellij-lsp-zed/
```

## Requirements

- **macOS**, **Linux**, or **Windows**
- **Zed** editor (any recent version)
- Internet connection on first launch

## Caveats

- **Library sources**: `Cmd+Click` into JDK/Spring classes won't open their
  source (Zed doesn't support `jar://` URIs yet). Navigation within your own
  code works fine.
- **First launch**: the initial project import can take a minute or two on
  large projects.

## License

[MIT](LICENSE) — the extension is MIT-licensed.

The IntelliJ LSP server itself is proprietary software by JetBrains, subject
to its own [EULA](https://www.jetbrains.com/legal/).
