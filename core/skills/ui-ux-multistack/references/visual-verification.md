# Visual verification

Writing UI code is not producing a good interface. Look at the rendered result.
Adapted from evondevKit's probe-based checks: render, measure what machines can
measure, then use eyes for the rest.

## Claim vocabulary (use exactly these levels)

| Level | Meaning | When allowed |
|---|---|---|
| **Source-reviewed** | Read the code, nothing rendered | Only source was inspected |
| **Rendered (partial)** | Rendered some screens or states or one appearance | Say which were not seen |
| **Rendered and inspected** | All specified screens, states, sizes, both appearances captured and examined | Captures exist and were viewed |
| **Measured** | Contrast, spacing, sizes, order measured with a tool | Numbers recorded |

Never write "verified", "looks good", or "pixel perfect" for a lower level. If a
renderer is unavailable, say "Source-reviewed only: no renderer available" and list
what the user should check.

## Tool matrix

Check availability first (`which xcodebuild playwright node`, available MCP tools).
Use what exists; do not install heavy tools without asking. A temporary directory for
helper installs is acceptable, never the user's project.

| Surface | Render | Capture | Inspect |
|---|---|---|---|
| SwiftUI | `#Preview`, `ImageRenderer`, build and run | `screencapture`, XCUITest screenshots | Accessibility Inspector, view hierarchy debugger |
| Web | dev server in a real browser | Playwright, browser automation, devtools screenshots | devtools, axe, Lighthouse, contrast tool |
| Hybrid | run the app | whole-window capture; web view capture separately | Web Inspector (inspectable web view) plus native inspector |
| Figma-sourced design | Figma MCP screenshot | design-to-code compare | variable definitions for tokens |

If the Yana environment exposes browser or native-UI capture tools, prefer them over
custom scripts.

## Capture matrix

At minimum for size S and above:

- Each main screen in its default state with realistic data.
- Empty, loading, error, long-content, many-items states.
- Light and dark (if both are supported).
- Narrow and wide: web 375 and 1440; native smallest allowed window and a large one.
- Keyboard focus visible on a Tab walk through the main task.
- Hybrid: live appearance switch and an accent change.

## What to measure

| Check | How | Fail |
|---|---|---|
| Contrast | Compute ratio for text/background and boundary pairs | Under 4.5:1 text, 3:1 UI |
| Spacing and size | Inspect computed values against the spec | More than 1 token step off |
| Overflow | Resize through widths; look for clipping and page scroll | Any clipped content or horizontal page scroll |
| Hit targets | Inspect bounding boxes | Under 24x24 CSS px on web |
| Focus order | Tab walk | Order differs from reading order, trap, invisible focus |
| Text wrap | Long and short strings | Buttons wrapping, numbers truncated |

## Compare against intent

Open the spec and the chosen wireframe beside the capture. List each deviation
(hierarchy, spacing, type, color, state) with severity. Fix defects; list taste items
separately. For redesigns, produce before and after captures with identical size,
data, and appearance.

## Report template

```
Verification level: Rendered and inspected (Measured for contrast)
Captured: screens A, B, C; states default, empty, error; light and dark; 1440 and 375
Not captured: <what and why>
Measurements: contrast pairs ..., min target ..., focus order ...
Deviations from spec: <list or none>
Open items: <list>
```

## Failure handling

If capture fails twice with the same error, stop retrying; report the blocker and the
level reached so far. If a measurement disagrees with what you see, trust the
measurement, then look again at why the eye was fooled.
