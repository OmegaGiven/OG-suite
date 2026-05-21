# Minimal UI Audit

This audit maps the current app surfaces against the OG rule that simple tools should avoid redundant labels, duplicate page titles, and explanatory text where the action or state is already clear.

## Current Pass

- Feed: removed the duplicated `Feed` page title from the app body and made the desktop refresh action icon-only.
- Notes: editor toolbar is mostly aligned with the rule because formatting actions are icon-only; file drawer still contains a few text states and status labels that should be reviewed after the save/connection control is standardized.
- Audio: recorder controls are mostly text-only, file actions are icon-only, and transcript/export actions now use text-only controls.
- Files: action bar is icon-only and compact; empty-state helper copy remains and should be shortened once the Drive backend exists.
- Admin: intentionally uses section headings and table labels because it is a management surface; still needs a later pass to avoid repeated page title language in mobile nav contexts.
- Settings/Appearance: theme, gradient, background image, font, reset, and done action buttons now use text-only controls; per-theme row actions remain icon-only.
- Suite auth/setup: page titles are necessary because the user is outside the main nav; keep them.

## Follow-Up Fixes

- Review empty-state copy across Feed, Files, Audio, and Notes after each app has the shared save/connection status control.
- Add a lightweight static audit script later if repeated regressions appear.
