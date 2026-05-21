# OG Suite Design Philosophy

## Goal
OG software should be low tech: able to run on slow machines with low memory use, low graphical cost, and relatively small app size.

The interface should be user-customizable. Users should be able to control:
- color scheme for backgrounds, panels/cards, text, nav bars, toolbars, and pop-ups
- margins
- corner rounding, including square corners
- panel opacity
- background image and background image opacity
- font selection
- gradients with editable color points, positions, strength, spread, and source count

Users should be able to save these settings as custom themes, export themes, and import themes. All apps should support dark mode, light mode, and custom themes.

Design should stay minimal. Most reasonable apps should fit on a phone screen, or have a version that can be ported to a phone screen cleanly.

## Never Use
- icon and text together for a simple button when one is enough
- a page title in the page when the nav bar already identifies the page
- descriptive text on self-explanatory tools or pages

## Local-First Model
All apps on desktop and mobile should save locally first. Backend hookup should be optional and should connect to an accompanying server that can back up local files whenever connection returns.

When server copies are newer, the app should save the newer server copy locally while keeping historical versions so users can audit and restore through the save/connection icon used by every app with these saving systems.

Each app should be a vertical slice that can run inside the full Suite app/server ecosystem or as a standalone app.

## Frontend
Preferred frameworks and languages:
- Tauri + Rust + Svelte
- Svelte alone for websites and web apps
- Tauri + Svelte for desktop and mobile apps using web UI

## Backend
- Rust

## Database
- PostgreSQL

## Apps To Design
- Suite: web/backend database that hosts current and future app designs.
- Notes: web/mobile/desktop app with concurrent editing.
- Audio recorder/transcriber: web/mobile app, with caption formatting for dropped-in videos and background cleaning.
- Feed: a canonical place to dump anything and everything, shown as a timeline. It can contain voice memos, notes, pictures, and videos. It should support metadata parsers and later LLM-assisted tagging for search and categorization.
- Files: a Google Drive-like app for uploading and managing files on the Suite server, including app-specific files grouped in their own sections, folder hierarchy, file types, sizes, created/modified dates, and search anywhere there is file management or formatting.
- Coms: communication app with messages, threads, custom thread categories, group/private messages, audio/video/screenshare calls, notifications, and per-app/per-thread/per-message notification controls.

## Common Element Rules
- Toolbars on mobile are side-swiping carousels.
- Toolbars on desktop wrap.
- Multi-tools open dropdowns. Example: a Header button opens a dropdown for heading sizes.
- Every section should be in a div separate from the whole app background so margin rules apply.
- Every color in the app should be customizable in the appearance menu.
- Appearance colors should be grouped cleanly with no more than three background/panel/nav/toolbar colors:
  - app background color
  - section element color
  - inner panel/tool color
- There should only be three text colors across the app, not including intentionally colored button text.
- Settings should show the current app/version.
