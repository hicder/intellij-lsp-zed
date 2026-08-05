#!/usr/bin/env bash
# =============================================================================
# intellij-lsp-install.sh — Download and prepare the IntelliJ LSP server for Zed
#
# Usage:
#   ./intellij-lsp-install.sh                          # Download latest from Open VSX
#   ./intellij-lsp-install.sh ./path/to/local.vsix     # Use a local VSIX file
#
# Env vars:
#   INTELLIJ_LSP_HOME    — Where to install the server (default: ~/.local/share/intellij-lsp)
#   INTELLIJ_LSP_CACHE   — Server cache/working dir   (default: ~/.cache/intellij-lsp-zed)
#   INTELLIJ_LSP_OUTPUT  — Where to write the instructions file (default: ./intellij-lsp-zed.md)
# =============================================================================

set -euo pipefail

# --- Configuration -----------------------------------------------------------
OPEN_VSX_URL="https://open-vsx.org/api/JetBrains/intellij-server/latest"
INSTALL_DIR="${INTELLIJ_LSP_HOME:-$HOME/.local/share/intellij-lsp}"
CACHE_DIR="${INTELLIJ_LSP_CACHE:-$HOME/.cache/intellij-lsp-zed}"
OUTPUT_FILE="${INTELLIJ_LSP_OUTPUT:-./intellij-lsp-zed.md}"

# --- Colors ------------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'
BOLD='\033[1m'; NC='\033[0m'

info()  { echo -e "  ${CYAN}[INFO]${NC}  $*"; }
ok()    { echo -e "  ${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "  ${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "  ${RED}[ERROR]${NC} $*"; }
bail()  { err "$*"; exit 1; }

# Portable mktemp (macOS compat)
make_tmp() {
    local dir="${TMPDIR:-/tmp}"
    mktemp -d "${dir}/intellij-lsp-install.XXXXXX"
}

# Portable sha256
sha256() {
    if command -v shasum &>/dev/null; then
        shasum -a 256 "$1" | awk '{print $1}'
    elif command -v sha256sum &>/dev/null; then
        sha256sum "$1" | awk '{print $1}'
    else
        bail "Neither shasum nor sha256sum found"
    fi
}

# --- Step 1: Obtain VSIX -----------------------------------------------------
echo ""
info "Step 1/6: Obtaining VSIX..."

VSIX_PATH=""
TMP_VSIX=""

cleanup_vsix() {
    [ -n "$TMP_VSIX" ] && rm -f "$TMP_VSIX" 2>/dev/null || true
}
trap cleanup_vsix EXIT

if [ $# -ge 1 ] && [ -f "$1" ]; then
    VSIX_PATH="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
    ok "Using local VSIX: $VSIX_PATH"
else
    TMP_VSIX="$(make_tmp)/intellij-server.vsix"
    mkdir -p "$(dirname "$TMP_VSIX")"
    info "Downloading latest VSIX from Open VSX..."

    DL_URL=$(curl -fsSL "$OPEN_VSX_URL" | python3 -c "
import sys, json
data = json.load(sys.stdin)
# Try 'download' field first (new API) then 'files.download'
d = data.get('download', '')
if not d and 'files' in data:
    d = data['files'].get('download', '')
if not d and 'allVersions' in data:
    # Get the latest version URL
    v = list(data['allVersions'].values())[0]
    d = v.split('@@')[0] if '@@' in str(v) else str(v)
print(d)
" 2>/dev/null)

    if [ -z "$DL_URL" ]; then
        # Fallback: try to grep the download URL
        DL_URL=$(curl -fsSL "$OPEN_VSX_URL" | grep -o '"https://open-vsx[^"]*\.vsix"' | head -1 | tr -d '"')
    fi

    if [ -z "$DL_URL" ]; then
        bail "Failed to resolve VSIX download URL from Open VSX"
    fi

    ok "Resolved: ${DL_URL:0:80}..."
    curl -#L -o "$TMP_VSIX" "$DL_URL" || bail "Download failed"
    VSIX_PATH="$TMP_VSIX"
    ok "VSIX downloaded ($(ls -lh "$VSIX_PATH" | awk '{print $5}'))"
fi

# --- Step 2: Extract VSIX & read server-bundle.json --------------------------
info "Step 2/6: Reading server bundle metadata..."

VSIX_TMP="$(make_tmp)"
unzip -o -q "$VSIX_PATH" -d "$VSIX_TMP" || bail "Failed to extract VSIX"

BUNDLE_JSON="$VSIX_TMP/extension/server-bundle.json"
if [ ! -f "$BUNDLE_JSON" ]; then
    # List what we got
    ls -la "$VSIX_TMP/extension/" 2>/dev/null || true
    bail "server-bundle.json not found in VSIX"
fi

SERVER_URL=$(python3 -c "import json; print(json.load(open('$BUNDLE_JSON'))['url'])" 2>/dev/null)
SERVER_SHA256=$(python3 -c "import json; print(json.load(open('$BUNDLE_JSON'))['sha256'])" 2>/dev/null)
SERVER_VERSION=$(python3 -c "import json; print(json.load(open('$BUNDLE_JSON'))['version'])" 2>/dev/null)
ARCHIVE_NAME=$(python3 -c "import json; print(json.load(open('$BUNDLE_JSON'))['archiveName'])" 2>/dev/null)

if [ -z "$SERVER_URL" ]; then
    # Fallback using grep
    SERVER_URL=$(grep -o '"url":"[^"]*"' "$BUNDLE_JSON" | head -1 | cut -d'"' -f4)
    SERVER_SHA256=$(grep -o '"sha256":"[^"]*"' "$BUNDLE_JSON" | cut -d'"' -f4)
    SERVER_VERSION=$(grep -o '"version":"[^"]*"' "$BUNDLE_JSON" | cut -d'"' -f4)
    ARCHIVE_NAME=$(grep -o '"archiveName":"[^"]*"' "$BUNDLE_JSON" | cut -d'"' -f4)
fi

ok "Server version: ${BOLD}$SERVER_VERSION${NC}"
ok "Archive name:  ${BOLD}$ARCHIVE_NAME${NC}"
ok "SHA256:        ${SERVER_SHA256:0:16}..."

# --- Step 3: Download (or use cached) server bundle ---------------------------
info "Step 3/6: Downloading server bundle..."

mkdir -p "$INSTALL_DIR"
SERVER_ZIP="$INSTALL_DIR/$ARCHIVE_NAME"

DOWNLOAD_NEEDED=true
if [ -f "$SERVER_ZIP" ]; then
    COMPUTED=$(sha256 "$SERVER_ZIP")
    if [ "$COMPUTED" = "$SERVER_SHA256" ]; then
        ok "Server bundle already cached (SHA256 verified)"
        DOWNLOAD_NEEDED=false
    else
        warn "Cached bundle SHA256 mismatch — re-downloading"
        rm -f "$SERVER_ZIP"
    fi
fi

if $DOWNLOAD_NEEDED; then
    info "Downloading from JetBrains CDN (~368 MB)..."
    curl -#L -o "$SERVER_ZIP" "$SERVER_URL" || bail "Download failed"
    COMPUTED=$(sha256 "$SERVER_ZIP")
    if [ "$COMPUTED" != "$SERVER_SHA256" ]; then
        rm -f "$SERVER_ZIP"
        bail "SHA256 verification failed! Expected: ${SERVER_SHA256:0:16}... Got: ${COMPUTED:0:16}..."
    fi
    ok "Server bundle downloaded ($(ls -lh "$SERVER_ZIP" | awk '{print $5}'))"
fi

# --- Step 4: Extract server --------------------------------------------------
info "Step 4/6: Extracting server..."

SERVER_DIR="$INSTALL_DIR/server-$SERVER_VERSION"

if [ ! -d "$SERVER_DIR" ]; then
    unzip -o -q "$SERVER_ZIP" -d "$INSTALL_DIR" || bail "Failed to extract server bundle"

    # The zip contains a subdirectory; find the real server dir
    EXTRACTED=$(find "$INSTALL_DIR" -maxdepth 2 -name "build.txt" -not -path "*/server-*" 2>/dev/null | head -1 | xargs dirname 2>/dev/null || true)

    if [ -z "$EXTRACTED" ]; then
        # Check if it was already extracted directly to the right place
        if [ -f "$SERVER_DIR/build.txt" ]; then
            EXTRACTED="$SERVER_DIR"
        fi
    fi

    if [ -z "$EXTRACTED" ] || [ ! -d "$EXTRACTED" ]; then
        # Try listing what we got
        ls -d "$INSTALL_DIR"/*/ 2>/dev/null || true
        bail "Could not find server directory in extracted bundle"
    fi

    if [ "$EXTRACTED" != "$SERVER_DIR" ]; then
        rm -rf "$SERVER_DIR" 2>/dev/null || true
        mv "$EXTRACTED" "$SERVER_DIR"
    fi
    ok "Server extracted to: $SERVER_DIR"
else
    ok "Server already extracted at: $SERVER_DIR"
fi

LAUNCHER="$SERVER_DIR/bin/intellij-server"
if [ ! -x "$LAUNCHER" ]; then
    bail "Launcher not found or not executable: $LAUNCHER"
fi

# --- Step 5: Compute EULA hash -----------------------------------------------
info "Step 5/7: Computing EULA hash..."

EULA_FILE="$SERVER_DIR/EULA.txt"
if [ ! -f "$EULA_FILE" ]; then
    EULA_FILE=$(find "$SERVER_DIR" -name "EULA.txt" -not -path "*/license/*" 2>/dev/null | head -1 || true)
fi
if [ ! -f "$EULA_FILE" ]; then
    warn "EULA.txt not found — using empty hash (server will reject this)"
    EULA_HASH="0000000000000000"
else
    # The EULA hash is the first 16 hex characters (64 bits) of SHA-256
    EULA_HASH=$(sha256 "$EULA_FILE" | cut -c1-16)
fi

ok "EULA hash:     ${BOLD}$EULA_HASH${NC}"

# --- Step 6: Generate Zed config snippet -------------------------------------
info "Step 6/7: Generating Zed configuration..."

mkdir -p "$CACHE_DIR"

cat <<SPLICE

╔═══════════════════════════════════════════════════════════════════════════╗
║  ${BOLD}Server installed at:${NC} ${SERVER_DIR}
║
║  ${BOLD}Next steps:${NC}
║    1. Install the extension (see intellij-lsp-zed.md)
║    2. Add to ${CYAN}~/.config/zed/settings.json${NC}:
║       ${BOLD}"Java"${NC}:   { "language_servers": ["intellij-server", "!jdtls"] }
║       ${BOLD}"Kotlin"${NC}: { "language_servers": ["intellij-server", "!kotlin-language-server"] }
║    3. ${CYAN}Cmd+Shift+P${NC} → ${CYAN}zed: reload${NC}
║
║  Run again: ${CYAN}./scripts/install.sh${NC}
╚═══════════════════════════════════════════════════════════════════════════╝

SPLICE

# --- Step 7: Write instructions file -----------------------------------------
info "Step 7/7: Writing instructions to $OUTPUT_FILE..."

cat > "$OUTPUT_FILE" <<MDEOF
# IntelliJ LSP for Zed — Setup

> Generated on $(date '+%Y-%m-%d %H:%M')
> Server: **${SERVER_VERSION}**

---

## Installation Data

| Item | Value |
|---|---|
| Server version | \`${SERVER_VERSION}\` |
| Launcher path | \`${LAUNCHER}\` |
| Cache / logs | \`${CACHE_DIR}\` |
| EULA hash | \`${EULA_HASH}\` |

---

## 1. Install the extension

Copy the extension files to Zed's extensions directory:

\`\`\`sh
mkdir -p ~/Library/Application\\ Support/Zed/extensions/installed/intellij-lsp-zed/
cp extension.wasm extension.toml \\
  ~/Library/Application\\ Support/Zed/extensions/installed/intellij-lsp-zed/
\`\`\`

On Linux, use \`~/.local/share/zed/extensions/installed/\`.

---

## 2. Configure \`~/.config/zed/settings.json\`

Add \`"language_servers"\` inside both \`"Java"\` and \`"Kotlin"\`:

\`\`\`json
"languages": {
  "Java": {
    "language_servers": ["intellij-server", "!jdtls"]
  },
  "Kotlin": {
    "language_servers": ["intellij-server", "!kotlin-language-server"]
  }
}
\`\`\`

> The \`!\` prefix disables Zed's default servers (\`jdtls\`, \`kotlin-language-server\`)
> to avoid duplicate diagnostics.

---

## 3. Restart Zed

\`Cmd+Shift+P\` → \`zed: reload\` (or close and reopen the editor).

Open a Java or Kotlin project. The server will auto-import Maven/Gradle/Bazel projects.

---

## Notes

- **Evaluation**: preview builds are valid for 30 days. Run \`./scripts/install.sh\` every 2 weeks to stay current.
- **Memory**: to increase the JVM heap, add \`"--jvm-arg=-Xmx4096m"\` to \`arguments\`.
- **Logs**: \`${CACHE_DIR}/system/log/intellij-server.log\`.
- **Clear cache**: delete \`${CACHE_DIR}/system/caches/\` if the index is corrupted.

---

## Full \`settings.json\` example

\`\`\`json
{
  "languages": {
    "Java": {
      "language_servers": ["intellij-server", "!jdtls"]
    },
    "Kotlin": {
      "language_servers": ["intellij-server", "!kotlin-language-server"]
    }
  }
}
\`\`\`
MDEOF

ok "Instructions written to ${OUTPUT_FILE}"

# --- Cleanup -----------------------------------------------------------------
rm -rf "$VSIX_TMP" 2>/dev/null || true

ok "Done!"
