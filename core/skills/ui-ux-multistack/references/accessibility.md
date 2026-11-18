# Accessibility (mandatory checks for every build)

Target WCAG 2.2 AA on web surfaces and the platform's accessibility conventions on
native ones. Automated tools catch roughly a third of issues; the manual checks below
are part of done.

## Shared checklist

| Check | Pass condition |
|---|---|
| Text contrast | 4.5:1 normal text, 3:1 large text (18.66 px bold or 24 px) |
| UI contrast | 3:1 for control boundaries, icons, focus indicators against neighbors |
| Color alone | Status also shown by icon, text, or pattern |
| Keyboard | Every action reachable and operable; no traps; logical order; visible focus |
| Names | Every control has an accessible name; icon-only controls labeled |
| Structure | Headings, landmarks (web) or grouping (native) reflect hierarchy |
| Targets | Web at least 24x24 CSS px; native standard control sizes |
| Motion | Reduced-motion respected; no flashing over 3 per second |
| Text size | Layout survives larger text (web zoom to 200 percent, native text size options) |
| Errors | Identified in text, tied to the field, with a way to fix |
| Time | No timeouts without warning and extension |

## Web specifics

- Use native elements; ARIA only to fill real gaps, and match the pattern's keyboard contract.
- Dialogs: focus moves in, is trapped, Esc closes, focus returns to the trigger.
- Live regions (`aria-live`) for async results and toasts, polite by default.
- `lang` on the document; `prefers-reduced-motion`, `prefers-contrast`, `forced-colors`.
- Form labels bound with `for`/`id`; required state communicated beyond an asterisk color.
- Skip link to main content on multi-region pages.

## Native macOS specifics

- `.accessibilityLabel`, `.accessibilityValue`, `.accessibilityHint`; combine compound
  rows with `.accessibilityElement(children: .combine)`; mark decorative images
  `.accessibilityHidden(true)`.
- Test with VoiceOver (Cmd-F5), Full Keyboard Access, and Accessibility Inspector.
- Honor `\.accessibilityReduceMotion`, `\.accessibilityReduceTransparency`,
  `\.colorSchemeContrast`, and `\.accessibilityDifferentiateWithoutColor`.
- Custom controls need traits, actions, and keyboard handling; prefer standard controls.

## Hybrid specifics

VoiceOver must move through native and web regions in a sensible order. Give the web
view an accessible label, expose landmarks inside it, and make sure bridge-driven
changes announce themselves. Check that focus returns to the right native control when a
web overlay closes.

## Manual pass (5 minutes)

1. Unplug the mouse: complete the main task with the keyboard alone.
2. Turn on a screen reader: do headings, labels, and states read sensibly?
3. Zoom or enlarge text to the maximum: nothing clipped or overlapping?
4. Switch to dark and to increased contrast: still readable, focus still visible?
5. Enable reduced motion: nothing essential depends on animation?

Record results in the delivery note under "Accessibility verified" with what was and
was not tested.
