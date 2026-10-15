import axe from "axe-core";

/**
 * Runs axe-core against a rendered node and throws a readable error on violations.
 *
 * Limits worth remembering: axe catches roughly a third of real-world a11y problems, and
 * `color-contrast` cannot run in jsdom (no layout/paint), so it is disabled here. Contrast is
 * checked in the browser (Storybook a11y panel) instead. Keyboard behavior, focus management and
 * screen reader output still need their own tests and manual checks.
 */
export async function expectNoA11yViolations(node: Element): Promise<void> {
  const results = await axe.run(node, {
    runOnly: {
      type: "tag",
      values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"],
    },
    rules: { "color-contrast": { enabled: false } },
  });

  if (results.violations.length > 0) {
    const report = results.violations
      .map((v) => {
        const targets = v.nodes.map((n) => `    ${n.target.join(" ")}`).join("\n");
        return `- ${v.id} (${v.impact}): ${v.help}\n${targets}\n  ${v.helpUrl}`;
      })
      .join("\n");
    throw new Error(`Found ${results.violations.length} accessibility violation(s):\n${report}`);
  }
}
