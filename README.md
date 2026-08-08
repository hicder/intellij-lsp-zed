# IntelliJ LSP for Zed

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Unofficial Zed extension that brings [IntelliJ IDEA's LSP server][1] for Java &
Kotlin to the [Zed editor][2] — code completion, navigation, refactorings,
inspections, and quick-fixes for Maven, Gradle, and Bazel projects.

> **Important — licensing.** The IntelliJ LSP server is **proprietary software
> by JetBrains** (not open source). Before the extension downloads or runs it,
> you must read and accept the [JetBrains EULA][3]. This extension never
> fetches the server from third-party registries (such as the Open VSX API):
> it either downloads the pinned build directly from JetBrains' CDN, or uses a
> server you downloaded yourself. See [License](#license).

[1]: https://blog.jetbrains.com/idea/2026/08/intellij-idea-goes-lsp/
[2]: https://zed.dev
[3]: https://www.jetbrains.com/legal/docs/toolbox/user/

## Install

1. Open Zed → `Cmd+Shift+P` → `zed: extensions` → search **IntelliJ LSP**
2. **Accept the JetBrains EULA** — add this to your Zed `settings.json`
   (`~/.config/zed/settings.json` on Linux/macOS):

   ```json
   {
     "lsp": {
       "intellij-server": {
         "settings": {
           "accept_jetbrains_eula": true
         }
       }
     }
   }
   ```

3. Open a Java or Kotlin project. The server (~368 MB) is installed once and
   reused on subsequent launches.

### Getting the server binary

How the server is obtained depends on the release:

- **Automatic (pinned build)** — the release pins a verified server build on
  JetBrains' own CDN and downloads it for you. This is the intended flow:
  accept the EULA (step 2) and you're done.
- **Manual** — if a release ships without a pin configured, the extension will
  tell you exactly what to set. Download the server from the [JetBrains
  announcement][1], extract it, and point the extension at the executable:

  ```json
  {
    "lsp": {
      "intellij-server": {
        "settings": {
          "accept_jetbrains_eula": true,
          "server_path": "/absolute/path/to/intellij-server/bin/intellij-server"
        }
      }
    }
  }
  ```

  `server_path` must point **directly at the `intellij-server` executable** (or
  `intellij-server.exe` on Windows) — the extension runs in a sandbox and
  cannot extract archives outside of it.

## Settings

All settings live under `lsp.intellij-server.settings` in your Zed
`settings.json`.

| Key                               | Type    | Required | Description                                                                                                                      |
| --------------------------------- | ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `accept_jetbrains_eula`           | boolean | yes      | Explicitly accept the JetBrains EULA. No download or execution happens unless this is `true`.                                    |
| `server_path`                     | string  | no       | Path to an already-extracted `intellij-server` executable (manual mode).                                                         |
| `server_version`                  | string  | no       | Override the pinned server version (automatic mode).                                                                             |
| `server_download_url`             | string  | no       | Override the pinned JetBrains download URL (automatic mode).                                                                     |
| `eula_hash`                       | string  | no       | EULA acceptance hash override (advanced — see Troubleshooting).                                                                  |
| `intellij.additionalJvmArgs`      | array   | no       | JVM options for the server process (e.g. `["-Xmx4g"]` to raise the 2 GB default heap).                                           |
| `intellij.dataSharing`            | string  | no       | `"full"` / `"anonymous"` / `"none"`. **Defaults to `none`** — independent consent, never inherited from `accept_jetbrains_eula`. |
| `intellij.region`                 | string  | no       | Region for JetBrains product terms / data processing.                                                                            |
| `intellij.projects`               | array   | no       | Monorepo project entries (`[{ "type": "gradle", "path": "file:///..." }]`).                                                      |
| `intellij.buildTool`              | string  | no       | Global build tool override (`"gradle"`, `"maven"`, `"bazel"`, or `""` to disable all).                                           |
| `intellij.jdkForSymbolResolution` | string  | no       | Path to a JDK home for symbol resolution.                                                                                        |

These keys are consumed by the extension and delivered to the server via
**initialization options** and **environment variables** (exactly as the real
JetBrains VS Code extension does).

## Advanced: JetBrains server settings

JetBrains' own VS Code extension delivers settings to the language server via
**initialization options** (`eulaHash`, `projects`, `buildTools`, `defaultSdk`)
and **environment variables** (`IJ_JAVA_OPTIONS`, `INTELLIJ_DATA_SHARING`,
`INTELLIJ_REGION`). This extension mirrors that behaviour 1:1 using the same
setting keys (dots included).

### Full example

A realistic `~/.config/zed/settings.json` using the IntelliJ server for Java
and Kotlin, with the EULA accepted, heap raised to 4 GB, data sharing kept
off, region set, and two monorepo sub-projects scoped for import:

```json
{
  "lsp": {
    "intellij-server": {
      "settings": {
        "accept_jetbrains_eula": true,
        "intellij.additionalJvmArgs": ["-Xmx4g", "-XX:+UseG1GC"],
        "intellij.dataSharing": "none",
        "intellij.region": "EU",
        "intellij.buildTool": "gradle",
        "intellij.projects": [
          { "type": "gradle", "path": "file:///Users/me/work/monorepo/module-a/build.gradle.kts" },
          { "type": "maven", "path": "file:///Users/me/work/monorepo/module-b/pom.xml" }
        ],
        "intellij.jdkForSymbolResolution": "/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home"
      }
    }
  },
  "languages": {
    "Java": {
      "language_servers": ["intellij-server", "!jdtls"]
    },
    "Kotlin": {
      "language_servers": ["intellij-server", "!kotlin-language-server"]
    }
  }
}
```

### Individual settings explained

Other useful ones:

- `intellij.additionalJvmArgs` — `["-Xmx4g"]` (→ `IJ_JAVA_OPTIONS`; default heap is 2 GB)
- `intellij.dataSharing` — `"full"` / `"anonymous"` / `"none"` (opt-in, **defaults to `none`**; see [Data sharing](#data-sharing))
- `intellij.region` — your region for JetBrains' product terms and data processing
- `intellij.buildTool` — `"gradle"` / `"maven"` / `"bazel"` (→ `buildTools` in init options)
- `intellij.jdkForSymbolResolution` — path to a JDK home (→ `defaultSdk` in init options)

See the [official IntelliJ LSP settings documentation][4] for the full list.

## Known limitations (Zed)

- **Run and debug are not available.** JetBrains runs and debugs through a
  custom protocol (`launch.json` "IntelliJ: Launch main class" / "Attach to
  JVM"), not the Debug Adapter Protocol that Zed supports. Tests are not
  supported by JetBrains yet either.
- **Live templates and file templates** are editor-side features in VS Code;
  they are not part of the LSP protocol.
- **One backend per workspace.** Only one IntelliJ server can access a
  workspace at a time — don't use VS Code and Zed on the same folder
  simultaneously.
- **Archive integrity verification not implemented at runtime.** The
  official sha256 hash for each platform's server archive (from
  `server-bundle.json`) is documented in code comments, but the extension
  does not verify it after downloading because `zed_extension_api` 0.7.0's
  `download_file` extracts the archive in-place and does not expose the raw
  bytes to the WASM sandbox for hashing. The archive is transported over
  HTTPS, and `download_file` reports HTTP errors; the pinned URL lives on
  JetBrains' own CDN. A future `zed_extension_api` release that supports raw
  download-then-extract would allow the extension to verify the sha256 before
  trusting the contents.

[4]: https://www.jetbrains.com/help/intellij-vscode/IntelliJ-lsp-settings.html
[5]: https://www.jetbrains.com/help/intellij-vscode/Project-import.html

## Disable Zed's built-in Java/Kotlin servers (optional)

The extension registers the IntelliJ server for Java and Kotlin automatically.
Zed also ships its own servers (`jdtls`, `kotlin-language-server`), so you'll
get duplicate diagnostics unless you disable them:

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

1. On every launch the extension checks that you accepted the JetBrains EULA
   (`accept_jetbrains_eula`). If not, it refuses to start and prints exactly
   what to add to `settings.json` — no download, no execution.
2. It checks its cache for an already-installed server and reuses it.
3. If none is installed, it downloads the pinned build directly from
   JetBrains' CDN (or uses your `server_path` / `server_version` +
   `server_download_url` overrides) and extracts it.
4. The EULA acceptance hash is computed from the bundled `EULA.txt` and passed
   to the server on startup.
5. Your project imports (Maven/Gradle/Bazel) and language features activate.

Cached versions are reused on subsequent launches.

## Evaluation & License

- During the preview the server is **free** — each build is valid for
  **30 days** from its release date
- After the preview, an IntelliJ IDEA Ultimate subscription will be required
- If the server stops working after ~30 days, install a newer build (clear the
  extension's cache, see Troubleshooting)

### Data sharing

JetBrains' own clients (VS Code, Cursor) additionally ask users to accept a
**data-sharing policy** and choose a region after installing the extension.
This extension keeps data sharing **disabled**: the server runs with data
sharing off (`dataSharing=NONE`) and no telemetry is sent to JetBrains.
Accepting the [EULA][3] is the only consent required.

## Troubleshooting

| Problem                                       | Fix                                                                                                                                                                                                                     |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| "you must read and accept the JetBrains EULA" | Add `"accept_jetbrains_eula": true` under `lsp.intellij-server.settings` (see [Install](#install)) and reload the window.                                                                                               |
| "no pinned automatic download configured"     | This release has no pinned server URL. Download the server from the [JetBrains announcement][1] and set `server_path`, or set `server_version` + `server_download_url`.                                                 |
| "Bundled license agreement is not accepted"   | The server reports the hash it expects (e.g. `expected hash 34d850193ee04897`). If you run the server from a manual `server_path`, copy that hash into the `eula_hash` setting. Automatic downloads compute it for you. |
| Server won't start / evaluation expired       | Clear the server cache: `rm -rf ~/Library/Application\ Support/Zed/extensions/work/intellij-lsp-zed` (Linux: `~/.local/share/zed/extensions/work/`, Windows: `%LOCALAPPDATA%\Zed\extensions\work\`), then reload Zed    |
| Download fails                                | Check your internet connection, then retry — the extension resumes cleanly                                                                                                                                              |
| Duplicate diagnostics                         | Add the `language_servers` config above to disable Zed's built-ins                                                                                                                                                      |

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

### Updating the pinned server

The extension deliberately does not resolve server versions from third-party
registries at runtime, so the maintainer pins a verified build in
`src/lib.rs` (`SERVER_VERSION`, `SERVER_DOWNLOAD_URL`, and optionally
`SERVER_EULA_HASH`). To update the pin after JetBrains releases a new build:

1. Download the new `JetBrains.intellij-server` VSIX once, from its page on
   Open VSX or the VS Code Marketplace, in a browser (one manual download —
   this is normal end-user usage, not an API query).
2. Extract the VSIX and read `extension/server-bundle.json`: it contains the
   real JetBrains CDN `url` and the server `version`.
3. Update the constants in `src/lib.rs`, rebuild `extension.wasm`, bump the
   version in `extension.toml` / `Cargo.toml`, and release.

The weekly [health-check workflow](.github/workflows/monitor.yml) verifies the
pinned URL is still alive and queries the Open VSX API once a week to detect
when a new build is available (maintainer dev-tooling, one request/week — not
the extension at runtime), opening an issue when an update is due.

## Requirements

- **macOS**, **Linux**, or **Windows**
- **Zed** editor (any recent version)
- Internet connection on first launch (automatic mode only)

## Caveats

- **Library sources**: `Cmd+Click` into JDK/Spring classes won't open their
  source (Zed doesn't support `jar://` URIs yet). Navigation within your own
  code works fine.
- **First launch**: the initial project import can take a minute or two on
  large projects.

## License

The extension code is [MIT](LICENSE).

The IntelliJ LSP server is proprietary software by JetBrains, subject to its
own [EULA][3]. It is **not** bundled with or redistributed by this extension —
it is downloaded from JetBrains after you explicitly accept the EULA, or used
from a path you provide. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
