# Native macOS: SwiftUI and AppKit

Check the project's deployment target before using an API. Gate newer APIs with
`#available`, and say which minimum OS a feature needs. For state and architecture see
`swiftui-patterns`; for Liquid Glass see `liquid-glass-design`. This file covers design.

## Principles (Apple Human Interface Guidelines, applied)

- Look like the platform: standard window chrome, toolbar, sidebar, menus, controls.
  Distinctiveness goes into content, accent, and information design.
- Mac users have a pointer, a keyboard, resizable windows, and multiple windows.
  Design for all four; do not port a phone layout.
- Every command is in the menu bar, and common ones have standard shortcuts.
- Respect system settings: accent color, appearance, reduced motion, reduced
  transparency, increased contrast, text size where supported.

## Window and structure

| Need | Use |
|---|---|
| Main window | `WindowGroup`; set `.defaultSize` and sensible minimums with `.frame(minWidth:minHeight:)` |
| Preferences | `Settings { }` scene, opened with Cmd-, ; tabs or sidebar by area |
| Utility or inspector panel | A separate `Window` or `.inspector(isPresented:)` (macOS 14+) |
| Sidebar + content (+ detail) | `NavigationSplitView`; `List` with `.listStyle(.sidebar)` gets vibrancy for free; `.navigationSplitViewColumnWidth(min:ideal:max:)` |
| Title and actions | `.navigationTitle`, `.toolbar { ToolbarItem(placement: .primaryAction) { } }`, `.searchable` |
| Hidden title bar | Only with a reason; keep window controls and a drag area intact |
| Document apps | `DocumentGroup` for real document models |

Do not build a custom sidebar from `VStack` plus backgrounds when `NavigationSplitView`
fits; you lose collapse, vibrancy, keyboard handling, and full screen behavior.

## Controls and layout

- `Form` with `.formStyle(.grouped)` for settings and editors; `LabeledContent` for
  label-value rows; `GroupBox` for related clusters; `Table` for multi-column data
  (`Table` supports sort descriptors and selection).
- `.controlSize(.small/.regular/.large)` rather than hand-sized frames.
- `Picker` styles: `.menu`, `.segmented`, `.radioGroup`; pick by option count and visibility.
- Use `Label` for icon plus text; `Image(systemName:)` with
  `.symbolRenderingMode(.hierarchical)` for SF Symbols. One symbol weight per toolbar.
- Spacing: 4-based steps; use default stack spacing unless the spec says otherwise.
- Leave native pointer behavior alone: buttons use the arrow cursor on macOS.

## Color, type, materials

- Prefer semantic colors (`.primary`, `.secondary`, `Color.accentColor`,
  `Color(nsColor: .controlBackgroundColor)`) so appearance and contrast adapt.
- Brand colors live in the asset catalog with Any/Dark (and high contrast) variants.
- Text styles (`.body`, `.headline`, `.title3`) over hard-coded sizes; macOS text
  styles are fixed point sizes, so do not assume iOS-style Dynamic Type scaling.
- Materials (`.regularMaterial`, `.thinMaterial`) only where layering needs depth:
  sidebars, floating panels, overlays. Not on every container.
- Respect `\.accessibilityReduceTransparency` by falling back to solid fills.

## Keyboard and focus

- `.keyboardShortcut`, `Commands`/`CommandMenu` for menu items, `@FocusState` for focus.
- Tab order follows reading order; arrows move within lists and tables;
  Return and Escape do what users expect in sheets and popovers.
- `.onKeyPress` (macOS 14+) for custom keys; do not override system shortcuts.

## Motion

Use `withAnimation(.easeOut(duration: 0.2))` or spring defaults for state changes.
Read `@Environment(\.accessibilityReduceMotion)` and swap movement for a fade.
No animation on every list update; animate the change the user caused.

## AppKit interop

| Need | Use |
|---|---|
| A control SwiftUI lacks | `NSViewRepresentable` with a coordinator |
| SwiftUI inside AppKit | `NSHostingView` or `NSHostingController` |
| Vibrancy on a custom view | `NSVisualEffectView` with a semantic material (`.sidebar`, `.headerView`, `.menu`) |
| Fine window control | `NSWindow` via a small window-accessor representable; keep it isolated |

Keep AppKit islands small and documented; they are where design drift starts.

## Accessibility (native)

`.accessibilityLabel/Value/Hint`, group with `.accessibilityElement(children: .combine)`,
test with VoiceOver and Keyboard Navigation, check `\.colorSchemeContrast`. Details in
`accessibility.md`.

## Verification (native)

- `#Preview` blocks for each state, light and dark, small and large window.
- `ImageRenderer` (macOS 13+) to render a view to an image without launching the app.
- Build and run, then capture the real window: `screencapture -l <windowID> out.png`
  (window id from `GetWindowID`-style helpers or `osascript`) or XCUITest screenshots.
- Xcode Accessibility Inspector for labels, contrast, hit regions.
Claim levels: `visual-verification.md`.

## Native pitfalls

Custom controls replacing standard ones; fixed frames that clip in other languages;
ignoring dark mode and increased contrast; toolbar stuffed with text buttons;
hidden title bar with no drag area; modal sheets for non-blocking tasks; hardcoded
shadows and blur that fight system materials; iOS patterns (tab bar, large titles) on Mac.
