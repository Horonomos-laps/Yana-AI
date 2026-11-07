# Web UI: HTML, CSS, JS/TS, React, Next.js, Vue, Tailwind

Follow the project's framework and styling convention. This skill sets tokens,
rhythm, structure, and states; it does not dictate how you write styles.

## Audit before writing

```bash
bash core/skills/ui-ux-multistack/scripts/detect-stack.sh .
```

Then check by hand: the styling approach (Tailwind, CSS Modules, styled-components,
plain CSS), a component library (shadcn/Radix, MUI, Ant, internal), the token file,
the i18n setup (copy language), and `DESIGN.md`. Report one line of findings before
designing. If a component library exists, use its components and tune tokens; do not
write a second Button.

## Stack notes

| Stack | Design-relevant facts |
|---|---|
| Plain HTML/CSS | Custom properties for tokens, `color-scheme`, semantic elements, no framework assumptions. Port component examples to markup, do not paste JSX |
| React | Components per state-bearing element; keep presentational components free of data fetching; stories or a specimen route for states |
| Next.js | App Router: server components by default, add `"use client"` only for interactivity; `next/font` for fonts, `next/image` for images; avoid layout shift |
| Vue/Nuxt | Single-file components, scoped styles, props for variants; Teleport for overlays |
| Tailwind | v3 configures in `tailwind.config.*`; v4 is CSS-first (`@import "tailwindcss"`, `@theme` in CSS). Check the version before touching config. Map semantic tokens to theme values, avoid arbitrary values scattered through markup |
| TypeScript | Type variant props as unions (`"primary" | "quiet"`), not free strings |

## Structure and semantics

- Landmarks: `header`, `nav`, `main`, `aside`, `footer`; one `h1` per view, headings in order.
- Real elements first: `button` for actions, `a` for navigation, `label` bound to inputs,
  `table` for tabular data, `ul` for lists. ARIA only when no element fits.
- Minimize wrapper elements; use CSS grid or flex for layout, not nested divs for spacing.

## Responsive

- Design at the narrowest and widest supported sizes first, then check between.
  Typical test widths: 375, 768, 1024, 1280, 1440, 1920.
- Prefer fluid layout: `grid` with `minmax`, `clamp()` for type and spacing,
  container queries (`@container`) for components that live in resizable panes.
- Use `dvh/svh` units for full-height layouts on mobile browsers.
- Tables on narrow screens: horizontal scroll with a visible affordance, or reflow to a list.
- No horizontal page scroll at any width; no text clipped by fixed heights.

## Interaction and states

- `:hover` only for pointer devices (`@media (hover: hover)`), `:focus-visible` for
  keyboard focus, `:active` for press, `[disabled]` and `aria-disabled` for disabled.
- Loading, empty, and error states are designed, not afterthoughts.
- Never `outline: none` without a replacement focus indicator.
- Targets at least 24x24 CSS px (WCAG 2.2 AA); aim for 40 or more for primary touch actions.
- Respect `prefers-reduced-motion`, `prefers-color-scheme`, `prefers-contrast`.

## Performance that users see

Reserve space for images and embeds (width/height or aspect-ratio); load fonts with
`font-display: swap` and subset them; avoid layout shift on data load; virtualize
long lists; keep animation to `transform` and `opacity`.

## Dark mode

Set `color-scheme: light dark`, define semantic tokens per scheme, test both.
If the app has a manual toggle, store the choice and apply it before first paint to
avoid a flash.

## Verification (web)

- Render in a real browser at the widths above, light and dark; capture screenshots.
- Playwright (or a browser automation tool) for captures and keyboard walks:
  Tab through the page, confirm order and visible focus.
- Measure contrast with devtools or a script; run axe or Lighthouse for a first
  accessibility pass, then do the manual checks axe cannot do.
- Stress with long strings, large numbers, empty data, slow loading.
Claim levels: `visual-verification.md`.

## Web pitfalls

Tailwind class soup with inconsistent spacing values; per-page ad hoc colors; icon
buttons with no accessible name; fixed pixel heights that clip translated text;
modals without focus trap and return; sticky headers hiding anchored content;
hover-only controls on touch; full-bleed gradients behind small text.
