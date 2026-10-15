# 0001: Foundations

Status: accepted (revisit items marked **open** once the issues log has real entries)

## Context

React + Tailwind component libraries are a crowded space (shadcn/ui, Radix, Base UI, React Aria
Components, Ark UI, Headless UI, HeroUI, Mantine, ...). A new one is only worth maintaining
publicly if it has a concrete reason to exist and a way to keep its accessibility claims honest.

## Decisions

1. **Accessibility is proven by tests, not asserted in docs.** Every component ships with:
   axe checks (jsdom, `src/test/a11y.ts`), keyboard-interaction tests, and an in-browser axe pass in
   Storybook (which includes color contrast, which jsdom cannot compute). Automated checks catch
   only a fraction of real problems, so components also get a short manual screen reader checklist
   (**open**: define and add under `docs/`).
2. **Every bug found in another library becomes a test here.** Findings go in
   `docs/issues-in-the-wild.md`, each linked to the test that guards against it. This is the
   differentiator; without it the project is another wrapper.
3. **Do not hand-roll hard interaction primitives.** Combobox, dialog focus trapping, menus,
   popover positioning, date pickers and the like are where accessible libraries accumulate years of
   edge-case fixes. **Open:** build on a headless layer (leaning React Aria Components for a11y/i18n
   depth; Base UI is the alternative with a more Tailwind-friendly API) versus own the primitives.
   Decide after the issues log shows what we would actually fix. Simple components (Button, Badge,
   Label, ...) stay plain elements.
4. **Distributed as an npm package, not copy-paste.** Accessibility fixes must reach consumers via a
   version bump; copied code drifts and never receives them. A shadcn-style registry can be added
   later on top of the package if people ask for it.
5. **Tailwind v4 only, tokens in CSS.** Semantic tokens live in `@theme` (`src/styles/theme.css`);
   consumers theme by overriding variables. No Tailwind v3 support, no JS theme config.
6. **React 19+ only.** `ref` is a regular prop, no `forwardRef`.
7. **Focus indicators use `outline`, not `box-shadow` rings**, so they survive forced-colors mode.
   Interactive targets are at least 24px (WCAG 2.2 target size).
8. **Tooling:** pnpm workspaces, tsdown (unbundled ESM output for per-component tree-shaking),
   Vitest + Testing Library + axe-core, Storybook (`addon-a11y`), Biome (includes a11y lint rules),
   Changesets. TypeScript 7 is used; tsdown warns that its API is experimental, so pin to
   TypeScript 5.9/6 if declaration output ever misbehaves.
9. **Only two workspaces** (`packages/ui`, `apps/storybook`). One package, many components. Split
   packages only when there is a real reason (for example a separate tokens package).

## Not yet decided

- npm scope and package name availability (`@crispy-potato` is a placeholder).
- License (a public repo with no license is not open source, so pick one before announcing).
- Docs site beyond Storybook.
- Real-browser keyboard/focus tests (Vitest browser mode or Playwright).
- Dark theme and the contrast-checking approach for token pairs.
