import { fireEvent, render, screen } from "@testing-library/svelte";
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

  it("closes on Escape when the question is empty", async () => {
    const onClose = vi.fn();
    render(InlineAsk, {
      side: "new",
      startLine: 10,
      endLine: 12,
      onAsk: vi.fn(),
      onClose,
    });

    await fireEvent.keyDown(
      screen.getByRole("textbox", { name: "Question about selected lines" }),
      { key: "Escape" },
    );

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("keeps the card open on Escape when the question has content", async () => {
    const onClose = vi.fn();
    render(InlineAsk, {
      side: "new",
      startLine: 10,
      endLine: 12,
      onAsk: vi.fn(),
      onClose,
    });
    const questionInput = screen.getByRole("textbox", {
      name: "Question about selected lines",
    });
    await fireEvent.input(questionInput, { target: { value: "Why?" } });

    await fireEvent.keyDown(questionInput, { key: "Escape" });

    expect(onClose).not.toHaveBeenCalled();
  });
});
