#!/usr/bin/env bash
# detect-stack.sh: report which UI technologies a project actually uses.
# Read-only. Never writes, installs, or executes project code.
#
# Usage: detect-stack.sh [project_dir]      (default: current directory)
# Exit:  0 report printed, 2 bad argument
set -euo pipefail

readonly MAX_DEPTH=6   # bounds scan time on large repos
readonly MAX_HITS=3    # evidence lines shown per signal, keeps output short

root="${1-.}"   # unset means cwd; an explicitly empty argument is an error
if [[ -z "$root" || ! -d "$root" ]]; then
  echo "detect-stack: not a directory: ${root:-<empty>}" >&2
  exit 2
fi
root="$(cd "$root" && pwd)"

# Directories never worth scanning (dependencies, build output, VCS).
prune_args=( \( -name node_modules -o -name .git -o -name .build -o -name Pods \
  -o -name DerivedData -o -name dist -o -name build -o -name .next -o -name target \
  -o -name .venv -o -name vendor \) -prune -o )

surfaces=()

# files_matching <name-glob> <regex>: files whose content matches, relative paths.
files_matching() {
  find "$root" -maxdepth "$MAX_DEPTH" "${prune_args[@]}" -type f -name "$1" \
    -exec grep -lE "$2" {} + 2>/dev/null | head -n "$MAX_HITS" || true
}

# named_paths <find -name glob> [type]: paths by name, relative evidence.
named_paths() {
  find "$root" -maxdepth "$MAX_DEPTH" "${prune_args[@]}" -name "$1" -print 2>/dev/null \
    | head -n "$MAX_HITS" || true
}

# report <surface> <evidence-lines>: record a surface and print its evidence.
report() {
  local label="$1" evidence="$2" line
  [[ -z "$evidence" ]] && return 0
  surfaces+=("$label")
  echo "SURFACE  $label"
  while IFS= read -r line; do
    [[ -n "$line" ]] && echo "         ${line#"$root"/}"
  done <<< "$evidence"
}

echo "PROJECT  $root"

# Native Apple stack.
report "swiftui"  "$(files_matching '*.swift' 'import SwiftUI')"
report "appkit"   "$(files_matching '*.swift' 'import AppKit|NSViewRepresentable|NSWindowController|NSHostingView')"
report "wkwebview" "$(files_matching '*.swift' 'WKWebView|import WebKit')"
bridge="$(files_matching '*.swift' 'WKScriptMessageHandler|messageHandlers|evaluateJavaScript')"
[[ -n "$bridge" ]] && { echo "BRIDGE   native<->web messaging found"; echo "$bridge" | sed "s|^$root/|         |"; }

# Web stack: read dependency names from package.json files.
pkgs="$(find "$root" -maxdepth 4 "${prune_args[@]}" -name package.json -print 2>/dev/null | head -n 8 || true)"
deps=""
if [[ -n "$pkgs" ]]; then
  while IFS= read -r pkg; do
    deps+="$(grep -hoE '"(react|next|vue|nuxt|svelte|@angular/core|tailwindcss|electron|@tauri-apps/api|vite|typescript|@radix-ui/[a-z-]+|@mui/material|antd)"' "$pkg" 2>/dev/null || true)"$'\n'
  done <<< "$pkgs"
fi
has_dep() { grep -qE "\"$1\"" <<< "$deps"; }
has_dep "react"                && surfaces+=("web-react")   && echo "SURFACE  web-react (package.json)"
has_dep "next"                 && surfaces+=("web-next")    && echo "SURFACE  web-next (package.json)"
has_dep "vue"                  && surfaces+=("web-vue")     && echo "SURFACE  web-vue (package.json)"
has_dep "tailwindcss"          && surfaces+=("tailwind")    && echo "SURFACE  tailwind (package.json)"
has_dep "electron"             && surfaces+=("electron")    && echo "SURFACE  electron (package.json)"
has_dep "@tauri-apps/api"      && surfaces+=("tauri")       && echo "SURFACE  tauri (package.json)"
has_dep "typescript"           && echo "NOTE     typescript present"

html_count="$(find "$root" -maxdepth "$MAX_DEPTH" "${prune_args[@]}" -type f -name '*.html' -print 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$html_count" -gt 0 && -z "$deps" ]]; then
  surfaces+=("plain-html"); echo "SURFACE  plain-html ($html_count .html files, no package.json deps)"
fi

# Design assets worth reusing.
echo "ASSETS"
for pattern in 'DESIGN.md' 'tokens.css' 'tailwind.config.*' '*.xcassets' 'Package.swift' '*.xcodeproj'; do
  hits="$(named_paths "$pattern")"
  [[ -n "$hits" ]] && while IFS= read -r p; do echo "         ${p#"$root"/}"; done <<< "$hits"
done
css_vars="$(files_matching '*.css' '^\s*--[a-z0-9-]+:')"
[[ -n "$css_vars" ]] && echo "         css custom properties in:" && echo "$css_vars" | sed "s|^$root/|           |"

# Verdict: native surface plus any web signal means hybrid.
native=0; web=0
for s in "${surfaces[@]:-}"; do
  case "$s" in
    swiftui|appkit) native=1 ;;
    wkwebview|web-*|tailwind|plain-html|electron|tauri) web=1 ;;
  esac
done
if [[ "$native" -eq 1 && "$web" -eq 1 ]]; then
  echo "HYBRID   yes (native surface plus web signals)"
elif printf '%s\n' "${surfaces[@]:-}" | grep -qE '^(electron|tauri)$'; then
  echo "HYBRID   yes (web content in a desktop shell)"
else
  echo "HYBRID   no"
fi
echo "SURFACES ${surfaces[*]:-none-detected}"
