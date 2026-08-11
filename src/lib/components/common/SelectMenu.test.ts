import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import SelectMenu from "./SelectMenu.svelte";

const options = [
  { value: "english", label: "English" },
  { value: "japanese", label: "日本語" },
];

describe("SelectMenu", () => {
  it("selects an option and closes the menu", async () => {
    const onChange = vi.fn();
    render(SelectMenu, {
      id: "language",
      value: "english",
      label: "Output language",
      options,
      onChange,
    });

    const trigger = screen.getByRole("combobox", { name: "Output language" });
    await fireEvent.click(trigger);
    const menu = screen.getByRole("listbox", {
      name: "Output language options",
    });
    await fireEvent.click(within(menu).getByRole("option", { name: "日本語" }));

    expect(onChange).toHaveBeenCalledWith("japanese");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("supports arrow keys and Escape", async () => {
    const onChange = vi.fn();
    render(SelectMenu, {
      id: "language",
      value: "english",
      label: "Output language",
      options,
      onChange,
    });

    const trigger = screen.getByRole("combobox", { name: "Output language" });
    await fireEvent.keyDown(trigger, { key: "ArrowDown" });
    await fireEvent.keyDown(trigger, { key: "ArrowDown" });
    await fireEvent.keyDown(trigger, { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith("japanese");

    await fireEvent.click(trigger);
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });
});
