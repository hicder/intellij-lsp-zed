<!-- Please confirm the checklist below, then remove this comment. -->

- [x] I've read [CONTRIBUTING.md](../CONTRIBUTING.md) and followed the relevant guidance for adding or updating my extension.

## Description

Adds **IntelliJ LSP** — a language server extension that brings JetBrains IntelliJ IDEA's Java and Kotlin intelligence to Zed.

JetBrains [recently announced](https://blog.jetbrains.com/idea/2026/08/intellij-idea-goes-lsp/) their IntelliJ LSP server for third-party editors (currently previewing in VS Code/Cursor). This is an **unofficial community port** of that server to Zed, using the same LSP protocol.

Features: code completion, navigation, refactorings, inspections, and quick-fixes for Maven, Gradle, and Bazel projects. Multi-platform (macOS, Linux, Windows).

- **Repository**: https://github.com/hlucas13/intellij-lsp-zed
- **Languages**: Java, Kotlin
- **License**: MIT (extension code) / JetBrains EULA (server, downloaded at runtime — not shipped with the extension)
- **EULA**: https://www.jetbrains.com/legal/docs/toolbox/user/
- **Server docs**: https://www.jetbrains.com/help/intellij-vscode/About-instance.html

### Compliance notes

- **No third-party registry APIs at extension runtime.** The extension itself never queries the Open VSX API (or any registry API) to resolve or download the server on a user's machine. It downloads a pinned, verified build directly from JetBrains' own CDN (`download.jetbrains.com`), and the version pin is in `src/lib.rs`. The maintainer's **weekly CI health check** (`monitor.yml`) does make a single Open VSX API request to detect new JetBrains builds — that's one request per week from the author's CI, run to tell the author when to update the pin. It does not scale with Zed's user base, does not touch the API from any end-user's machine, and is not involved in resolving what gets downloaded (the pin is static and pre-committed).
- **Explicit EULA consent gate.** The IntelliJ LSP server is proprietary (JetBrains EULA). The extension refuses to download or run anything until the user reads the EULA and opts in via `"lsp": { "intellij-server": { "settings": { "accept_jetbrains_eula": true } } }`. The EULA link is surfaced in the first-run error message and the README, and the consent flag is re-checked on every launch.
- **Settings delivered via init options + env vars.** JetBrains settings (`intellij.projects`, `intellij.buildTool`, `intellij.jdkForSymbolResolution`, `intellij.additionalJvmArgs`, `intellij.dataSharing`) are mapped to the same initialization options and environment variables the real VS Code extension uses — matching its [documented settings](https://www.jetbrains.com/help/intellij-vscode/IntelliJ-lsp-settings.html) 1:1.
- **Data sharing is opt-in and defaults to off.** `intellij.dataSharing` (`"full"` / `"anonymous"` / `"none"`) is a completely independent consent axis from EULA acceptance. It defaults to `none` (telemetry disabled), matching JetBrains' own client behaviour which also frames data collection as "with your consent."
