# Anti-patterns and self-check

Use during review and before delivering. Each row: the tell, why it hurts, the fix.

## Visual

| Anti-pattern | Why it hurts | Fix |
|---|---|---|
| Grid of equal cards for everything | No hierarchy, forces scanning | Table, list with detail, split view, or one hero plus support |
| Gradient for decoration | Reads as generic, lowers text contrast | Flat surface; reserve gradient for brand moments that carry meaning |
| Glass or blur everywhere | Cost in legibility and performance, unclear layering | Material only for floating layers and sidebars |
| Five to eight type sizes in one panel | Hierarchy dissolves | Four sizes, weight and color for the rest |
| Random spacing values | Looks unfinished | Scale-based spacing, audited by token |
| Inconsistent radius | Parts look like different products | Two or three radii, nested radii computed |
| Accent on every control | Accent stops signaling | Accent for primary action, selection, focus, links |
| Shadow on every container | Visual noise | Shadow only on floating layers |
| Text over busy imagery | Fails contrast | Solid scrim or move text |
| Emoji or icons as decoration | Noise | Icons that carry meaning, one family |

## UX

| Anti-pattern | Fix |
|---|---|
| Modal for a long task | Page, panel, or sheet with room and a way out |
| Settings scattered across modals | One settings area grouped by user goals |
| No empty, loading, or error design | Specify each state; build static examples |
| Placeholder as label | Persistent label |
| Destructive action with no undo or confirm | Undo toast or confirmation naming the object |
| Icon-only controls with unclear meaning | Add label or tooltip, accessible name |
| Dashboard with no action path | Each metric links to the thing you can do about it |
| Onboarding of seven steps | Three steps, explain permissions in context, allow skipping |

## Native macOS

| Anti-pattern | Fix |
|---|---|
| Phone layout on Mac | Sidebar, toolbar, multiple windows, keyboard |
| Custom button styles replacing standard | Native controls, tune tint and size |
| Pointer-hand cursor on buttons | Default arrow like system controls |
| Toolbar of long text buttons | Icon items with labels in customization, key actions only |
| Hidden title bar with no drag area | Keep a drag area or use unified toolbar |
| Ignoring dark mode or increased contrast | Semantic colors, test appearances |

## Web

| Anti-pattern | Fix |
|---|---|
| `outline: none` without replacement | `:focus-visible` style that meets 3:1 |
| Hover-only affordances | Visible on focus and touch too |
| Div soup with click handlers | Real `button` and `a` elements |
| Fixed heights that clip text | Min-height and flexible layout |
| Layout shift on load | Reserve space for images and data |
| Tailwind arbitrary values everywhere | Map to theme tokens |

## Hybrid

Two token sets that drifted; web chrome mimicking native chrome badly; theme set at
launch only; shortcuts swallowed by the web view; web view used for sidebar.

## Process anti-patterns

- Designing before finding the stack, then rewriting for the real one.
- Building a new component when one exists.
- Claiming verification from source reading.
- Redesigning everything when three rows were asked.
- Hedged final answers ("it depends") with no verdict.
- Copying a reference app's look instead of learning its structure and state handling.

## Self-check before delivery

1. Did I run the detector and read one real file per surface?
2. Did I reuse existing tokens and components?
3. Is there one primary action and one primary content area per screen?
4. Are all states designed and shown?
5. Are contrast and target sizes measured, not guessed?
6. Does the claim level match what I actually did?
7. Did I stay inside the approved scope?
