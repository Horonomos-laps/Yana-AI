# Design system intelligence

Create, extend, or reuse. Reuse first: grep for existing tokens before inventing any.

## 1. Token layers

```
primitive   raw values            blue-600 = #2563EB, space-4 = 16
semantic    roles                 color.accent, color.surface, color.text.muted
component   per-control choices   button.primary.bg -> color.accent
```

Components reference semantic tokens only. Primitives are referenced only by semantic
tokens. This is what makes light/dark and re-theming one change instead of fifty.

## 2. Scales (starting points; adopt the project's own if it has them)

| Token | Scale |
|---|---|
| Spacing | 4-based: 2, 4, 8, 12, 16, 24, 32, 48, 64 |
| Radius | 4, 6, 8, 12, 16, full. Pick 2 to 3 for the whole app; larger controls get larger radius |
| Type (web) | 12, 13, 14, 16, 20, 24, 32. Body 14 or 16; line height 1.4 to 1.6 for text, 1.2 to 1.3 for headings |
| Type (macOS) | Use text styles (`.body`, `.headline`, `.title3`); body is 13 pt by platform convention |
| Elevation | none, raised (menus, popovers), overlay (modals). Shadows only on layers that float |
| Motion | 100 to 150 ms hover and press, 200 to 300 ms enter and exit, 350+ only for large panels |
| Density | Compact, regular, spacious as spacing multipliers, not separate designs |

Limit type sizes to what a screen needs: four or five distinct sizes, not eight.
Weight and color carry hierarchy as much as size.

## 3. Color

- Roles: background, surface, surface-raised, border, border-strong, text, text-muted,
  text-on-accent, accent, accent-subtle, success, warning, danger, info, focus.
- One neutral ramp plus one accent plus status colors. More colors need a reason.
- Contrast targets (WCAG 2.2 AA): text 4.5:1, large text and UI boundaries 3:1.
  Measure; do not eyeball. Status must never rely on color alone (add icon or text).
- **Dark mode is a separate design, not an inversion.** Re-pick accent lightness,
  express elevation by lighter surfaces instead of shadows, avoid pure black on pure
  white text (halation), re-check every contrast pair.
- Accent appears where it means something: primary action, selection, focus, links.
  If accent is on every element it identifies nothing.

## 4. Component specification (write per component)

```
Name, purpose, anatomy
Variants (max 4 for buttons: primary, default, quiet, destructive)
Sizes
States: default, hover, pressed, focus, disabled, loading, selected, error, empty
Content rules: min/max length, truncation, empty text
Behavior: keyboard, focus order, motion
Tokens used
```

Destructive actions: distinct but not shouting; confirmation or undo for data loss.

## 5. Reuse and extend rules

1. Search the codebase for the thing you are about to build (component name, token name).
2. Found: use it. If it breaks a rule, adjust its tokens, do not fork it.
3. Library present (shadcn, Radix, MUI, native SwiftUI controls): use it and tune
   tokens; limit yourself to the variants the spec allows.
4. Missing: build one component to the spec, name it like its neighbors.
5. If the project has `DESIGN.md`, new decisions go there in the same change.

## 6. One source of truth across technologies

For hybrid or multi-platform products, keep tokens in one file and generate the rest.

```json
{ "color": { "accent": { "light": "#2563EB", "dark": "#7AA2FF" } },
  "space": { "2": 8, "3": 12 }, "radius": { "control": 8 } }
```

Generate: CSS custom properties for web, an asset catalog or `Color` extension for
SwiftUI, and theme JSON for shells that need it (Code-OSS color themes). A small
script or Style Dictionary works; do not hand-copy values between platforms.
Map names, not just values, so `color.accent` exists everywhere.

## 7. Specimen page

Before screens (sizes M and L), build one specimen: every token, every component,
every state, light and dark. On web it is a route; in SwiftUI it is a `#Preview` or a
debug window. Review the specimen first; screens inherit its decisions.

## 8. Token snippets

```css
:root {
  --color-surface: #ffffff; --color-text: #1d1d1f; --color-accent: #2563eb;
  --space-2: 8px; --space-3: 12px; --radius-control: 8px;
  color-scheme: light dark;
}
@media (prefers-color-scheme: dark) {
  :root { --color-surface: #1c1c1e; --color-text: #f5f5f7; --color-accent: #7aa2ff; }
}
```

```swift
enum Space { static let s2: CGFloat = 8, s3: CGFloat = 12 }
enum Radius { static let control: CGFloat = 8 }
// Colors: define "Accent", "Surface" in Assets.xcassets with Any/Dark variants,
// then use Color("Accent"), or keep system semantic colors where native fit matters.
```

## 9. Maintaining it

Every new value must map to an existing token or add one with a stated reason.
Flag one-off literals during review. Remove unused tokens at refactor time, not
during a design task unless asked.
