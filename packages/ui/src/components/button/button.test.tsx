import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { Button } from "./button";

describe("Button", () => {
  it("exposes the button role with its accessible name", () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });

  it('defaults to type="button" so it never submits a form by accident', () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole("button")).toHaveAttribute("type", "button");
  });

  it("activates with Enter and Space", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    render(<Button onClick={onClick}>Save</Button>);

    await user.tab();
    expect(screen.getByRole("button")).toHaveFocus();
    await user.keyboard("{Enter}");
    await user.keyboard(" ");
    expect(onClick).toHaveBeenCalledTimes(2);
  });

  it("does not fire when disabled", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    render(
      <Button disabled onClick={onClick}>
        Save
      </Button>,
    );
    await user.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });

  it("lets a consumer className override built-in utilities", () => {
    render(<Button className="h-20">Save</Button>);
    const button = screen.getByRole("button");
    expect(button).toHaveClass("h-20");
    expect(button).not.toHaveClass("h-10");
  });

  it.each(["primary", "secondary", "ghost"] as const)(
    "has no axe violations (%s)",
    async (variant) => {
      const { container } = render(<Button variant={variant}>Save</Button>);
      await expectNoA11yViolations(container);
    },
  );
});
