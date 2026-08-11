import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import DiffFileList from "./DiffFileList.svelte";

describe("DiffFileList", () => {
  it("selects the requested changed file", async () => {
    const onSelect = vi.fn();
    const { container } = render(DiffFileList, {
      files: [
        {
          oldPath: "src/review.ts",
          newPath: "src/review.ts",
          status: "modified",
          additions: 3,
          deletions: 1,
        },
      ],
      onSelect,
    });

    await fireEvent.click(screen.getByRole("button", { name: /review\.ts/i }));
    expect(onSelect).toHaveBeenCalledWith("src/review.ts");
    expect(screen.queryByText("M")).not.toBeInTheDocument();
    expect(screen.queryByText("+3")).not.toBeInTheDocument();
    expect(screen.queryByText("−1")).not.toBeInTheDocument();
    expect(
      container.querySelector(".file-row .lucide-file-diff"),
    ).toBeInTheDocument();
  });

  it("filters by the complete repository path", async () => {
    render(DiffFileList, {
      files: [
        {
          newPath: "src/alpha.ts",
          status: "added",
          additions: 1,
          deletions: 0,
        },
        {
          newPath: "tests/beta.ts",
          status: "added",
          additions: 1,
          deletions: 0,
        },
      ],
      onSelect: vi.fn(),
    });

    await fireEvent.input(screen.getByLabelText("Filter changed files"), {
      target: { value: "tests/" },
    });
    expect(screen.queryByText("alpha.ts")).not.toBeInTheDocument();
    expect(screen.getByText("beta.ts")).toBeInTheDocument();
  });

  it("collapses and expands directory branches", async () => {
    render(DiffFileList, {
      files: [
        {
          newPath: "src/lib/context.ts",
          status: "added",
          additions: 2,
          deletions: 0,
        },
        {
          newPath: "src/services/review.ts",
          status: "modified",
          additions: 3,
          deletions: 1,
        },
      ],
      onSelect: vi.fn(),
    });

    expect(screen.getByText("context.ts")).toBeInTheDocument();
    expect(screen.getByText("review.ts")).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Collapse src" }));
    expect(screen.queryByText("context.ts")).not.toBeInTheDocument();
    expect(screen.queryByText("review.ts")).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Expand src" }));
    expect(screen.getByText("context.ts")).toBeInTheDocument();
    expect(screen.getByText("review.ts")).toBeInTheDocument();
  });
});
