# Hybrid desktop: native shell with embedded web

Many desktop apps mix technologies: a SwiftUI sidebar and toolbar around a WKWebView
workspace, an Electron or Tauri app, a Code-OSS fork with custom views. Do not assume
one technology. Find each surface and decide which layer owns what.

## Ownership rule

| Layer | Owns |
|---|---|
| Native shell | Window chrome, toolbar, menu bar, sidebar, settings, system dialogs, global shortcuts |
| Web content | Rich, fast-changing, cross-platform workspace content (editors, canvases, markdown, dashboards) |
| Shared | Tokens (color, type, spacing, radius, motion), iconography language, copy tone |

If a native control exists for the job, use it. Put web where web is the stronger tool,
not where it is convenient for the team.

## Detecting the topology

`detect-stack.sh` reports HYBRID when it finds a native surface plus web signals.
Then confirm by reading: where is the web view created, what does it load (bundled
files, local server, remote URL), and what crosses the bridge.

## Native and web communication

WKWebView (macOS/iOS):
- Web to native: `WKScriptMessageHandler`; JS calls
  `window.webkit.messageHandlers.<name>.postMessage(payload)`.
- Native to web: `evaluateJavaScript` or injected `WKUserScript`.
- Local content: `loadFileURL(_:allowingReadAccessTo:)` or a custom scheme via
  `setURLSchemeHandler`; avoid broad file read access.
- Debugging: `isInspectable = true` (macOS 13.3+) in debug builds only.
- In SwiftUI the view is wrapped with `NSViewRepresentable`; keep the coordinator the
  single place that owns the bridge.

Electron: preload script with `contextBridge.exposeInMainWorld`; no node integration
in the renderer. Tauri: commands and events through the invoke API. Code-OSS: contribute
through its theme and extension points; its workbench exposes `--vscode-*` CSS variables.

Design the bridge messages as a small typed protocol (`{ type, payload }`), including
errors. Treat web content as untrusted input; validate every inbound message.

## Visual consistency across the boundary

1. **One token source.** Generate CSS variables and native assets from the same file
   (`design-system.md` section 6). Names match across platforms.
2. **Theme sync.** WKWebView follows the app appearance for `prefers-color-scheme`.
   Push accent and any custom tokens at load and on change (native injects CSS
   variables). Test the live switch, not only the first load.
3. **Type.** Use the system font stack inside web (`-apple-system, system-ui`) when the
   shell is native, unless the brand font is a deliberate identity choice used in both layers.
4. **Density and radius.** Match control heights and corner radii of the native
   neighbors; a 14 px web button beside a 22 pt native control looks wrong.
5. **Icons.** Pick one family. If native uses SF Symbols, web uses a matching weight
   and stroke set, or exported SVGs of the same symbols where licensing allows.
6. **Cursor and selection.** Controls use the default arrow like native ones; allow
   text selection in content, disable it on chrome (`user-select: none`).
7. **Scroll.** Avoid nested scrollers fighting each other and double rubber-banding;
   one scroll container per region.

## Window, viewport, focus

- Resizing: the web view fills its container and reflows; test the smallest window
  and a split-screen width. Set sensible minimum window sizes in the shell.
- Safe areas and toolbars: with transparent or full-size toolbars, add top insets
  so web content does not slide under controls.
- Dragging: web content does not move a window by default. Electron uses
  `-webkit-app-region: drag` and `no-drag` on interactive children; Tauri uses a drag
  region attribute. In a pure WKWebView, keep the native toolbar as the drag area.
- First responder and shortcuts: a focused web view consumes keystrokes. Route global
  shortcuts through native `Commands` or the menu, and forward only the keys the
  page should own. Verify Cmd-C/V/Z/F inside and outside the web view.
- Focus order: Tab must cross the boundary in both directions without traps.
- Context menu: replace default web menus where they expose irrelevant items.

## Platform notes

- Electron: `titleBarStyle: "hiddenInset"`, `vibrancy`, and traffic-light position give
  a native feel; check fullscreen and window restoration.
- Tauri: overlay title bar styles with a drag region; test on each target OS.
- Code-OSS forks: color changes flow through theme JSON and CSS variables; design
  custom views as workbench-native (same fonts, densities, codicons) so they read as
  part of the editor, then layer product identity through tokens, not bespoke chrome.

## Verification (hybrid)

Capture the whole window, not just the web view: native and web must be judged
together. Check light, dark, a live appearance switch, narrow window, accent change,
high contrast, VoiceOver traversal across the boundary, and keyboard shortcuts at the
boundary. Report separately what was verified in each layer.

## Hybrid pitfalls

Two design systems that drifted; web buttons beside native ones with different height
and radius; pointer-hand cursor in chrome; web scrollbars styled differently from
native; theme flash on launch; shortcuts swallowed by the web view; text clipped under
a transparent toolbar; a web view used for a sidebar that should be native; no
bridge error handling so the UI silently stalls.
