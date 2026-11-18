---
name: ui-ux-multistack
description: >
  Design, review, and implement application UI across native macOS (SwiftUI, AppKit),
  web (HTML, CSS, TypeScript, React, Next.js, Vue, Tailwind), and hybrid desktop apps
  that embed web views (WKWebView, Electron, Tauri, Code-OSS forks). Detects the real
  UI technology of each surface first, then runs an adaptive design workflow: discovery,
  analysis, direction, wireframes, design spec, build, rendered verification, refinement.
  Use when asked to "design a screen", "redesign this app", "improve the UI/UX", "build
  a component", "create a design system", "review this screenshot", "fix visual
  inconsistencies", "make this look native", or "evaluate usability".
  Do NOT use for SwiftUI state and architecture (see swiftui-patterns), Liquid Glass API
  details (see liquid-glass-design), a pure accessibility audit (see accessibility), or
  a one-line style tweak that needs no design judgment.
origin: adapted:evondev/evondevKit
license: MIT (c) 2026 Tuấn Trần (evondevKit methodology); Yana AI additions Apache-2.0
version: 1.0.0
compatibility: "macOS SwiftUI/AppKit, any web frontend stack, hybrid desktop shells. Visual verification uses whatever renderer the machine has (Xcode, Playwright, browser MCP, screencapture)."
---

# UI/UX Multi-Stack Design

A design skill, not a code generator. It makes the agent decide like a product designer,
then build with the technology the project actually uses. One application can hold
several UI technologies at once; this skill treats each surface on its own terms and
keeps one visual language across them.

Methodology adapted from evondevKit (MIT). See `references/attribution.md`.

## Core rules (apply to every task)

1. **Find the real stack before designing.** Run the detector (Step 1). Never assume a
   macOS app is all SwiftUI or a "web" project is all React.
2. **Reuse what exists.** Existing tokens, components, and conventions beat new ones.
   A second competing system is a defect, not an improvement.
3. **Native where the platform expects native.** Sidebars, toolbars, menus, settings,
   and window chrome on macOS stay native even when content is web.
4. **Design the states, not just the happy screen.** Default, hover, pressed, focus,
   disabled, loading, empty, error, long content, narrow window.
5. **Challenge weak decisions.** If a request or existing design is the problem, say
   so, explain why, and offer a better option. Do not silently comply, do not lecture.
6. **Verify the rendered result, or say you could not.** Reading code is not seeing
   the interface. Use the claim vocabulary in `references/visual-verification.md`.
7. **Scale the process to the task.** A one-component fix does not get eight phases.

## Step 1: Identify the technology of each surface

```bash
bash core/skills/ui-ux-multistack/scripts/detect-stack.sh <project_dir>
```

The script is read-only. It reports surfaces (SwiftUI, AppKit, WKWebView, React, Next,
Vue, Tailwind, Electron, Tauri, plain HTML), design assets (DESIGN.md, token files,
asset catalogs), and a HYBRID verdict. Then confirm by opening one real file per
surface; the script finds signals, it does not read intent.

| Surface found | Load |
|---|---|
| SwiftUI / AppKit window, sidebar, toolbar, settings | `references/macos-swiftui.md` |
| HTML, CSS, TS, React, Next, Vue, Tailwind | `references/web-ui.md` |
| WKWebView, Electron, Tauri, Code-OSS inside a native shell | `references/hybrid-desktop.md` (plus both above) |
| Anything else (Flutter, Qt, Compose, ...) | Inspect the code, apply `references/design-system.md` principles, follow that framework's own idioms. Do not port SwiftUI or CSS habits onto it. |

## Step 2: Route the request

| Request | Path |
|---|---|
| New screen, new app, redesign | Full workflow below, sized by Step 3 |
| "Review", screenshot critique, "what is wrong with this" | `references/workflow.md` section "Review mode": findings table first, no edits until the user picks rows |
| Establish or extend a design system | `references/design-system.md`, then build a specimen page/preview before any screen |
| Single component or bug in the look | Phase 6 and 7 only, reuse existing tokens |
| Keep brand, only tidy | Keep frame and color roles, replace off-system controls, change nothing else |
| Usability evaluation | Heuristic pass in `references/design-intelligence.md`, then evidence-ranked findings |

## Step 3: Size the task, then run the phases

| Size | Example | Phases run |
|---|---|---|
| XS | one component, one color, one spacing bug | 2 (brief look), 6, 7 |
| S | one screen | 1 to 7, wireframes optional when the layout is dictated by a pattern |
| M | a flow or several screens | all eight, wireframes required, one shared spec |
| L | whole app or redesign | all eight, design system first, gated per area |

| Phase | Output | Detail |
|---|---|---|
| 1 Discovery | Brief: product, users, main job per screen, constraints | `workflow.md` |
| 2 Analysis | Stack, existing system, audited problems by category | `design-intelligence.md` |
| 3 Direction | One chosen aesthetic direction with reasons | `aesthetic-directions.md` |
| 4 Layout | Hierarchy, 2 to 3 genuinely different wireframes | `workflow.md`, `components-patterns.md` |
| 5 Specification | Tokens, type, color, components, states, motion | `design-system.md` |
| 6 Implementation | Code in the project's real stack and conventions | platform references |
| 7 Verification | Rendered captures, measured checks, honest claim level | `visual-verification.md` |
| 8 Refinement | Fix real defects found in 7, no unrequested redesign | `workflow.md` |

**Two human gates** (sizes M and L, or S when direction is genuinely open): approve the
brief, then choose a wireframe. Between gates, do not stop to ask; take a sensible
default and state it in the delivery note. If the user says "just build it", skip the
wireframe gate and build the option you would have recommended.

## Distinguish the kind of problem

Label every finding as one of: **product** (wrong thing to build), **UX** (task
is hard to complete), **visual** (hierarchy, spacing, type, color), **technical**
(rendering, layout bug, performance), or **taste** (a preference with no evidence
either way). Fix product and UX first. Never present taste as a defect.

## What good looks like

- Hierarchy readable in three seconds: one primary action, one primary content area.
- Spacing and type come from a scale, not from guesses.
- Color carries meaning (state, emphasis, identity), not decoration.
- The app looks like it belongs to its platform and to its product.
- Dense where users work all day, calm where users decide once.

## Output Contract

Every review, critique, or recommendation ends with a forced conclusion. "It depends"
and "consider your needs" may appear in reasoning, never as the final answer.

```
Verdict:     [specific decision: ship / fix rows N,M first / redesign area X]
Confidence:  [0-100%, with the reason it is not higher]
Evidence:    [files read, captures taken, measurements, which are rendered vs source-only]
Next action: [one concrete step]
```

## Verification Step (numbers must be measured)

Before finalizing any build or review:
1. Contrast: measure text/background pairs. Body text below 4.5:1, large text or UI
   boundaries below 3:1 is a defect, not a note. Do not estimate by eye.
2. Spacing, radius, and type sizes in the render must match the spec; deviation over
   1px (or 1 token step) is flagged, not silently corrected.
3. Tap/click targets: below 24x24 CSS px (web) is a defect; native controls use the
   platform's standard control sizes.
4. Cross-check at least two window sizes and both appearances when both are supported.
5. If a measurement could not be taken, label the result "unverified", confidence 60%
   or lower, and say which check is missing.

## Hard avoid list

Generic dashboards of equal-weight cards; decorative gradients and glass with no
function; inconsistent radius and spacing; five font sizes within one panel; custom
controls that replace native ones with no gain; web-style pointer-hand and focus rings
in a native shell; visual complexity standing in for quality. Full list with fixes:
`references/anti-patterns.md`.

## Deliverables

- **Brief and direction** (short, in chat or `DESIGN.md` if the project uses one).
- **Wireframe options** as ASCII or rendered mockups, each with a trade-off line.
- **Design spec**: tokens, type scale, component list with states.
- **Implementation** in the real stack.
- **Delivery note**: stack found, system reused, defaults chosen, verification level
  reached, open items. If the project keeps `DESIGN.md`, record new decisions there.

## Boundaries

- Do not write application logic (API calls, persistence). Leave typed handlers or
  stubs for the user to connect.
- Do not install dependencies without asking; propose with one-line reasons.
- Do not edit files outside the declared scope; report scope growth before acting.
- Never claim visual verification that did not happen.

## Reference index

| File | Load when |
|---|---|
| `references/workflow.md` | Running the phases, review mode, gates |
| `references/design-intelligence.md` | Classifying problems, challenging decisions, heuristics |
| `references/aesthetic-directions.md` | Choosing a visual direction |
| `references/design-system.md` | Tokens, type, color, spacing, states, light/dark |
| `references/components-patterns.md` | Layout and component choices per screen type |
| `references/macos-swiftui.md` | Native macOS work |
| `references/web-ui.md` | Web work |
| `references/hybrid-desktop.md` | Native shell with embedded web |
| `references/accessibility.md` | Any build; mandatory checks |
| `references/visual-verification.md` | Phase 7; claim vocabulary |
| `references/anti-patterns.md` | Review and self-check |
| `references/attribution.md` | License and provenance |
