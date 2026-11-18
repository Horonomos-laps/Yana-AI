# Aesthetic directions

Pick one primary direction per product from the brief, not from habit. The idea of a
named set of "faces" is adapted from evondevKit (ten dashboard faces); the set and
criteria here cover native, web, and hybrid products.

## How to choose

1. Answer the product-fit questions in `design-intelligence.md` section 9.
2. Shortlist two directions. Prefer the one that matches the platform and the user's
   working posture (all-day tool vs occasional visit).
3. Choose one, write two reasons tied to the brief, and one thing it rules out.
4. Pick a **tone** axis separately: Neutral (color only for state), Tinted (accent
   washes on selection and key areas), Vivid (accent and gradients are part of the
   brand). Default to Neutral. Escalate only when the brand or the user asks.
5. Record the choice in `DESIGN.md` or the delivery note so later work stays consistent.

## Directions

| Direction | Signature | Choose when | Avoid when |
|---|---|---|---|
| **Native quiet** | System fonts, system materials, SF Symbols, semantic colors, standard controls | macOS utilities, productivity, anything that should feel built into the OS | The brand needs a strong distinctive identity |
| **Dense utility** | Small type, tight rows, strong dividers, tables as the hero, restrained color | Logs, ledgers, ops consoles, IDE panels, all-day expert tools | Overview or onboarding screens, casual users |
| **Soft tinted** | Light tinted surfaces, generous radius, accent washes for selection | Consumer-friendly productivity, wellness, notes | Data-dense work |
| **Midnight** | Dark first, careful elevation by lightness, one saturated accent | Developer tools, media, monitoring, night use | Document-like reading, print-adjacent tasks |
| **Paper** | Near-white canvas, hairline rules, serif or humanist accents, little chrome | Writing, reading, knowledge tools | Heavy control surfaces |
| **Editorial** | Large type contrast, asymmetry, strong grid, restrained imagery | Content-led products, reports, portfolios | Dense forms and tables |
| **Technical mono** | Monospace for data and labels, grid-aligned, high precision feel | Developer and security tools, terminals, telemetry | Warm consumer products |
| **Rail and compact** | Icon rail, collapsible panels, many tools in little space | Creative tools, multi-panel workspaces | Users who need labels to learn the app |
| **Float** | Inset panels on a tinted canvas, soft shadows on transient layers only | Modern SaaS shells with few big blocks | Many small adjacent blocks (edges blur) |
| **Instrument panel** | Gauges, status colors, strong numerics, clear states | Monitoring, hardware, trading, runtime control | Narrative content |

Each direction fixes: type pairing, radius range, border vs fill separation, shadow
policy, density, and how accent is used. Write those six values into the spec.

## Platform adjustments

- **macOS native**: default to Native quiet unless the product identity argues
  otherwise. A distinctive direction should live in content and accent, not in
  replacing window chrome, sidebars, and controls.
- **Web**: any direction works; check it against responsive behavior and keyboard use.
- **Hybrid**: choose the direction once and express it through shared tokens
  (`design-system.md`) so the native shell and the web surface read as one product.

## Anti-generic checks

A direction is generic if you cannot say what it rules out. Reject these defaults:
purple-to-blue gradient hero, equal-weight card grid, glass on everything, one accent
used for every button, icon in a colored circle for every list item. Replace the
filler with a real structural choice: a table, a split view, a timeline, a canvas.

## Motion tone

Match the direction: Native quiet and Dense utility use short, functional transitions
(roughly 120 to 200 ms, ease-out). Soft and Editorial can use slightly longer, softer
curves. No direction justifies motion that blocks input or ignores reduced-motion.
