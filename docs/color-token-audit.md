# Color Token Audit

Run:

```bash
npm run audit:colors
```

The scanner checks app/UI source files for literal `hex`, `rgb`, and `hsl` values outside the canonical token source. Literal values in `packages/ui/src/index.ts` are expected because that file defines the theme tokens.

## Current Findings

The current audit reports no literal color values outside tokenized expressions.

## Remediation Direction

- Keep actual theme default literals only in `packages/ui/src/index.ts`.
- Re-run `npm run audit:colors` after UI color changes.
- Add any new semantic colors to the shared token contract before using them in app styles.
