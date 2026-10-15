# Crispy Potato

Accessible React + Tailwind UI components, with accessibility treated as the product rather than a
checklist item.

> **Status: pre-alpha.** The repo is bootstrapped and one placeholder component (`Button`) exists to
> prove the toolchain. Nothing is published yet and the API will change.

## Why another UI library?

The headless layer (focus management, keyboard behavior, ARIA wiring) is largely a solved problem
upstream. The gaps are in the details that surface when real products use these libraries. This
project exists to track and fix those, and to prove each fix with a test. See
[`docs/issues-in-the-wild.md`](docs/issues-in-the-wild.md) and
[`docs/decisions/0001-foundations.md`](docs/decisions/0001-foundations.md).

## Repo layout

```
packages/ui        @crispy-potato/ui, the published library
apps/storybook     Storybook: visual development + in-browser a11y checks
docs/              decisions (ADRs) and the issues-in-the-wild log
```

## Development

Requires Node >= 22.12 and pnpm 10.

```sh
pnpm install
pnpm dev              # Storybook on http://localhost:6006
pnpm check            # lint + typecheck + test + build (what CI runs)
pnpm test             # unit + axe tests
pnpm format           # apply Biome formatting and lint fixes
pnpm changeset        # describe a user-facing change for the changelog
```

## Using the library (once published)

```css
/* app.css, Tailwind v4 */
@import "tailwindcss";
@import "@crispy-potato/ui/theme.css";
@source "../node_modules/@crispy-potato/ui/dist";
```

```tsx
import { Button } from "@crispy-potato/ui";
```
