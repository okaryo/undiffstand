import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import InlineAsk from "./InlineAsk.svelte";

describe("InlineAsk", () => {
  it("focuses the question input when it opens", () => {
    render(InlineAsk, {
      side: "new",
      startLine: 10,
      endLine: 12,
      onAsk: vi.fn(),
      onClose: vi.fn(),
    });

    expect(
      screen.getByRole("textbox", { name: "Question about selected lines" }),
    ).toHaveFocus();
  });
});
