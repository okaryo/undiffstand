import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { RepositoryInfo } from "$lib/domain/project";
import ProjectDialog from "./ProjectDialog.svelte";

const repository: RepositoryInfo = {
  repoPath: "/repos/example",
  suggestedName: "example",
  detectedBaseRef: "main",
  currentBranch: "feature",
  recentBranches: ["main"],
  localBranches: ["feature", "main"],
  remoteBranches: ["origin/main"],
  recentCommits: [],
};

describe("ProjectDialog", () => {
  it("saves the automatically detected base branch", async () => {
    const onSave = vi.fn();
    render(ProjectDialog, {
      repository,
      onSave,
      onClose: vi.fn(),
    });

    expect(
      screen.getByRole("combobox", { name: "Base branch" }),
    ).toHaveTextContent("main");
    await fireEvent.click(screen.getByRole("button", { name: "Add project" }));

    expect(onSave).toHaveBeenCalledWith({
      id: undefined,
      name: "example",
      repoPath: "/repos/example",
      baseRef: "main",
    });
  });

  it("requires a base branch when detection fails", async () => {
    const onSave = vi.fn();
    render(ProjectDialog, {
      repository: {
        ...repository,
        detectedBaseRef: null,
        localBranches: ["feature", "develop"],
      },
      onSave,
      onClose: vi.fn(),
    });

    const saveButton = screen.getByRole("button", { name: "Add project" });
    expect(saveButton).toBeDisabled();
    expect(
      screen.getByText(
        "Base branch could not be detected. Select it to continue.",
      ),
    ).toBeInTheDocument();

    await fireEvent.click(
      screen.getByRole("combobox", { name: "Base branch" }),
    );
    await fireEvent.click(
      within(
        screen.getByRole("listbox", { name: "Base branch options" }),
      ).getByRole("option", {
        name: "develop",
      }),
    );
    expect(saveButton).toBeEnabled();
    expect(
      screen.queryByText(
        "Base branch could not be detected. Select it to continue.",
      ),
    ).not.toBeInTheDocument();

    await fireEvent.click(saveButton);
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({ baseRef: "develop" }),
    );
  });
});
