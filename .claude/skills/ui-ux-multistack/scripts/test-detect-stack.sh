#!/usr/bin/env bash
# test-detect-stack.sh: fixture tests for detect-stack.sh (SwiftUI, web, hybrid, edges).
# Usage: bash test-detect-stack.sh        Exit 0 all pass, 1 any failure.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
detector="$here/detect-stack.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/detect-stack-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT

pass=0; fail=0
# expect <name> <needle> <haystack>: assert haystack contains needle.
expect() {
  if grep -qF -- "$2" <<< "$3"; then pass=$((pass + 1)); echo "ok    $1"
  else fail=$((fail + 1)); echo "FAIL  $1 (missing: $2)"; echo "$3" | sed 's/^/        /'; fi
}
refute() {
  if grep -qF -- "$2" <<< "$3"; then fail=$((fail + 1)); echo "FAIL  $1 (unexpected: $2)"
  else pass=$((pass + 1)); echo "ok    $1"; fi
}

# Fixture 1: pure SwiftUI app.
mkdir -p "$work/swift/Sources/App" "$work/swift/Assets.xcassets"
printf 'import SwiftUI\nstruct V: View { var body: some View { Text("x") } }\n' > "$work/swift/Sources/App/V.swift"
touch "$work/swift/Package.swift"
out="$("$detector" "$work/swift")"
expect "swiftui detected"        "SURFACE  swiftui" "$out"
refute "swiftui not hybrid"      "HYBRID   yes" "$out"
expect "asset catalog listed"    "Assets.xcassets" "$out"

# Fixture 2: web app with React, Tailwind, tokens.
mkdir -p "$work/web/src"
printf '{"dependencies":{"react":"18","tailwindcss":"4","typescript":"5"}}\n' > "$work/web/package.json"
printf ':root {\n  --color-accent: #2563eb;\n}\n' > "$work/web/src/tokens.css"
out="$("$detector" "$work/web")"
expect "react detected"          "web-react" "$out"
expect "tailwind detected"       "tailwind" "$out"
expect "css variables listed"    "tokens.css" "$out"
refute "web not hybrid"          "HYBRID   yes" "$out"

# Fixture 3: hybrid, SwiftUI shell with WKWebView and a message bridge.
mkdir -p "$work/hybrid/Sources" "$work/hybrid/web"
printf 'import SwiftUI\nimport WebKit\nfinal class C: NSObject, WKScriptMessageHandler {}\n' > "$work/hybrid/Sources/Shell.swift"
printf '<html></html>\n' > "$work/hybrid/web/index.html"
out="$("$detector" "$work/hybrid")"
expect "hybrid verdict"          "HYBRID   yes" "$out"
expect "wkwebview detected"      "wkwebview" "$out"
expect "bridge detected"         "BRIDGE" "$out"

# Fixture 4: Electron shell is hybrid even with no Swift.
mkdir -p "$work/electron"
printf '{"devDependencies":{"electron":"30"}}\n' > "$work/electron/package.json"
out="$("$detector" "$work/electron")"
expect "electron hybrid"         "HYBRID   yes" "$out"

# Edge cases.
out="$("$detector" "$work/does-not-exist" 2>&1)"; code=$?
expect "missing dir message"     "not a directory" "$out"
[[ "$code" -eq 2 ]] && { pass=$((pass + 1)); echo "ok    missing dir exits 2"; } || { fail=$((fail + 1)); echo "FAIL  missing dir exit=$code"; }

out="$("$detector" "" 2>&1)"; code=$?
[[ "$code" -eq 2 ]] && { pass=$((pass + 1)); echo "ok    empty arg exits 2"; } || { fail=$((fail + 1)); echo "FAIL  empty arg exit=$code"; }

mkdir -p "$work/empty dir with spaces"
out="$("$detector" "$work/empty dir with spaces")"
expect "empty dir reports none"  "none-detected" "$out"

mkdir -p "$work/inj"
touch "$work/inj/a;touch pwned.swift"
"$detector" "$work/inj" > /dev/null
[[ ! -e "$work/pwned.swift" && ! -e "$PWD/pwned.swift" ]] && { pass=$((pass + 1)); echo "ok    shell metachar filename not executed"; } || { fail=$((fail + 1)); echo "FAIL  metachar filename executed"; }

mkdir -p "$work/nm/node_modules/pkg"
printf 'import SwiftUI\n' > "$work/nm/node_modules/pkg/X.swift"
out="$("$detector" "$work/nm")"
refute "node_modules pruned"     "SURFACE  swiftui" "$out"

echo "----"; echo "passed: $pass  failed: $fail"
[[ "$fail" -eq 0 ]]
