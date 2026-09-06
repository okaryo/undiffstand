import { describe, expect, it, vi } from "vitest";
import type { DiffSelection } from "$lib/domain/diff";
import { DiffWorkspaceController } from "./diff-workspace.svelte";

function createController() {
  return new DiffWorkspaceController(
    {
      getDiffWorkspace: vi.fn(
        async (_projectId: string, selection: DiffSelection) => ({
          summary: {
            selection,
            comparison: {
              fromLabel: selection.base,
              toLabel: selection.target,
            },
            files: [],
            totalAdditions: 0,
            totalDeletions: 0,
          },
          reviewAvailability: { available: true, scopeLabel: "Changes" },
        }),
      ),
      getComparisonCommits: vi.fn(async () => []),
      getFileDiffs: vi.fn(async () => []),
    },
    vi.fn(),
    vi.fn(),
  );
}

describe("DiffWorkspaceController reviewed files", () => {
  it("keeps review state while refreshing the same comparison", async () => {
    const controller = createController();
    controller.activate("project");
    controller.setReviewed("src/review.ts", true);

    await controller.load();

    expect(controller.reviewedPaths.has("src/review.ts")).toBe(true);
  });

  it("clears review state when the comparison or scope changes", async () => {
    const controller = createController();
    controller.activate("project");
    controller.setReviewed("src/review.ts", true);

    await controller.applySelection({ base: "main", target: "HEAD" });
    expect(controller.reviewedPaths.size).toBe(0);

    controller.setReviewed("src/review.ts", true);
    await controller.applyScope({ kind: "commit", sha: "abc123" });
    expect(controller.reviewedPaths.size).toBe(0);
  });
});
