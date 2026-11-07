# Components and layout patterns

Choose the structure from the job of the screen, then the components from the data.
Concrete component code lives in the platform references; this file decides what to build.

## Screen type to default structure

| Screen job | Default structure | Notes |
|---|---|---|
| Browse many items, act on one | Sidebar or filter, list or table, detail pane or inspector | macOS: NavigationSplitView; web: master-detail, collapse to stack when narrow |
| Operate on data in bulk | Table with sortable columns, row selection, bulk bar, filters above | Dense; sticky header; keyboard row navigation |
| Edit one record | Form grouped by meaning, primary action at the end, cancel always available | Group under 5 to 7 fields per section, labels above or aligned, inline errors |
| Change preferences | Settings scene (macOS) or grouped settings page | Tabs or sidebar by area; changes apply immediately unless risky |
| Understand status now | Instrument panel: few large numbers, state colors, trend only where it drives action | Not a wall of equal cards |
| Start or onboard | Short linear steps, one decision per step, skip and back always available | Max 3 to 5 steps; explain permission requests before they appear |
| Find anything | Command palette or global search, results grouped by type, keyboard first | Cmd-K on web, Cmd-F or searchable on macOS |
| Read or write long content | Single column, comfortable measure (60 to 75 characters), quiet chrome | Hide controls until needed |
| Chat or agent session | Message stream, composer pinned, tool/status rows collapsed by default | Show running, done, failed clearly; long output collapsible |
| Review changes | Side-by-side or unified diff, file list, summary on top | Keyboard next/previous |

## Instead of a card grid

Equal-weight cards say "nothing here matters more." Use, in order of fit:
a table (comparable records), a list with one expanded row (browse and act), a split
view (select and inspect), a timeline (events), a canvas (spatial work), or one
hero metric with supporting detail. Use cards only for items that are genuinely
independent, self-contained, and varied in content.

## Core components: decisions

| Component | Rule |
|---|---|
| Button | Four dialects at most: primary (one per view), default, quiet, destructive. Label with a verb |
| Input | Visible boundary or fill; label always present; error text under the field; do not use placeholder as label |
| Select/Picker | Use native control on macOS; on web prefer native `<select>` unless search or rich options are required |
| Tabs vs segmented | Tabs switch views; segmented controls switch a mode of the same view |
| Modal/Sheet | Only for short, blocking, focused tasks. Never stack modals. Esc closes, focus returns to the trigger |
| Popover/Menu | Transient, anchored, dismissed on outside click and Esc |
| Toast | For results of actions already done; undo when possible; never for errors that need action |
| Banner | Persistent condition that affects the whole view |
| Empty state | Say what goes here, why it is empty, and the one action to fill it |
| Loading | Skeleton for layout-stable content, spinner for short waits, progress with numbers for long ones |
| Badge/Tag | Status or category only; color plus text; keep to one row |
| Table | Right-align numbers, left-align text, truncate with tooltip, sticky header, row hover distinct from selection |
| Tree | Disclosure arrows, keyboard arrows, clear selected vs hovered |
| Icon | One style family (SF Symbols on macOS); icon plus label unless universally known |

## State matrix (every interactive component)

Default, hover, pressed, focus-visible, disabled, loading, selected, error. Plus for
lists and views: empty, partial, long content, many items, offline. Specify the
visual change for each, and build each as a static example for review.

## Content rules

- Real copy, not lorem ipsum. Use the product's language and the user's locale.
- Test with long names (60+ characters), big numbers, missing values, and RTL if relevant.
- Truncate in the middle for paths and ids, at the end for prose.
- Dates and numbers formatted by locale, never hardcoded.

## Layout rules

- Align to a grid; align labels and values on shared edges.
- Group by proximity first, dividers second, containers last.
- Keep one primary action visible without scrolling.
- Use whitespace for grouping, not decoration. Do not fill space because it exists.
- Pick one container radius and nest radii consistently (inner = outer minus padding).
