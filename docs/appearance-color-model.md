# Appearance Color Model

OG Suite uses a three-layer surface model and a three-layer text model. Existing legacy token names remain available while apps are migrated.

## Surface Tokens

- App background: `colorBackground`, emitted as `--og-bg` and `--bg`.
- Section surface: `colorSection`, emitted as `--og-section-bg` and `--section-bg`.
- Inner panel/tool/nav surface: `colorPanel`, emitted as `--og-panel-bg` and `--inner-panel-bg`.

## Text Tokens

- Primary text: `colorText`, emitted as `--og-text` and `--text`.
- Muted text: `colorMuted`, emitted as `--og-muted` and `--muted`.
- Inverse/contrast text: `colorTextInverse`, emitted as `--og-text-inverse`.

## Compatibility Tokens

The older surface tokens are still normalized and emitted while apps are moved over:

- `colorSurface`, `colorSurfaceSubtle`, and `colorSurfaceStrong`
- `colorToolBackground`
- `colorActionBarBackground`
- `colorNav`

New UI should prefer the three-layer model. Existing UI can keep compatibility tokens until each app is audited and simplified.
