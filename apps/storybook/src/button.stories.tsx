import { Button } from "@crispy-potato/ui";
import type { Meta, StoryObj } from "@storybook/react-vite";

const meta = {
  title: "Components/Button",
  component: Button,
  args: { children: "Button" },
  argTypes: {
    variant: { control: "inline-radio", options: ["primary", "secondary", "ghost"] },
    size: { control: "inline-radio", options: ["sm", "md", "lg"] },
  },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
export const Secondary: Story = { args: { variant: "secondary" } };
export const Ghost: Story = { args: { variant: "ghost" } };
export const Disabled: Story = { args: { disabled: true } };
