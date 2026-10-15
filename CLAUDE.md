# Crispy Potato

Accessible React 19 + Tailwind v4 component library. pnpm workspace: `packages/ui` (published) and
`apps/storybook`. Read `docs/decisions/0001-foundations.md` before making structural decisions.

## Commands

- `pnpm check`: lint + typecheck + test + build (run before committing)
- `pnpm test`: Vitest (jsdom). `pnpm dev`: Storybook. `pnpm format`: Biome autofix.

## Conventions

- One folder per component in `packages/ui/src/components/<name>/` with `<name>.tsx`,
  `<name>.test.tsx`, `index.ts`; story in `apps/storybook/src/<name>.stories.tsx`; export from
  `packages/ui/src/index.ts`.
- Every component needs: an axe test (`expectNoA11yViolations`), keyboard interaction tests, and a
  story. Color contrast is checked in Storybook, not jsdom.
- Use semantic tokens from `theme.css` (`bg-primary`), never raw colors. Focus styles use
  `outline`, not box-shadow rings.
- Consumers' `className` must win: always merge with `cn()`.
- Add a changeset (`pnpm changeset`) for user-facing changes to `packages/ui`.
- When you find a bug in another library worth avoiding, log it in `docs/issues-in-the-wild.md`.
