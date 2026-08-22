import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DiffSelection, DiffSummary, FileDiff } from "$lib/domain/diff";
import type { ProjectConfig } from "$lib/domain/project";
import Page from "./+page.svelte";

const tauriApi = vi.hoisted(() => {
  const api = {
    listProjects: vi.fn(),
    touchProject: vi.fn(),
    saveProjectComparison: vi.fn(),
    getDiffSummary: vi.fn(),
    getFileDiffs: vi.fn(),
    validateRepository: vi.fn(),
    selectRepository: vi.fn(),
    saveProject: vi.fn(),
    removeProject: vi.fn(),
    getUserPreferences: vi.fn(),
    saveUserPreferences: vi.fn(),
    explainFileChange: vi.fn(),
    askInlineQuestion: vi.fn(),
    getChangeReviewAvailability: vi.fn(),
    runChangeReview: vi.fn(),
  };
  return {
    ...api,
    getDiffWorkspace: (projectId: string, selection: DiffSelection) =>
      Promise.all([
        api.getDiffSummary(projectId, selection),
        api.getChangeReviewAvailability(projectId, selection),
      ]).then(([summary, reviewAvailability]) => ({
        summary,
        reviewAvailability,
      })),
  };
});
const notifyReviewComplete = vi.hoisted(() => vi.fn());
const updater = vi.hoisted(() => ({
  check: vi.fn(),
  downloadAndInstall: vi.fn(),
  relaunch: vi.fn(),
  onFocusChanged: vi.fn(),
}));

vi.mock("$lib/services/tauri", () => ({ tauriApi }));
vi.mock("$lib/services/notification", () => ({ notifyReviewComplete }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: updater.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: updater.relaunch }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ onFocusChanged: updater.onFocusChanged }),
}));

const project: ProjectConfig = {
  id: "alpha",
  name: "Alpha",
  repoPath: "/repos/alpha",
  baseRef: "main",
  comparison: { base: "HEAD", target: "." },
  lastOpenedAt: "2026-08-08T10:00:00Z",
};

const summary: DiffSummary = {
  selection: { base: "HEAD", target: "." },
  comparison: {
    fromLabel: "HEAD",
    toLabel: "working tree",
    fromSha: "1234567890abcdef",
  },
  totalAdditions: 2,
  totalDeletions: 1,
  files: [
    {
      oldPath: "src/example.ts",
      newPath: "src/example.ts",
      status: "modified",
      additions: 2,
      deletions: 1,
    },
  ],
};

const fileDiff: FileDiff = {
  file: summary.files[0],
  oldContent: "const answer = 41;\n",
  newContent: "const answer = 42;\n",
  unifiedDiff:
    "diff --git a/src/example.ts b/src/example.ts\n--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-const answer = 41;\n+const answer = 42;\n",
  truncated: false,
};

class IntersectionObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const originalScrollIntoView = Element.prototype.scrollIntoView;
const originalGetContext = HTMLCanvasElement.prototype.getContext;

describe("change details auto-refresh", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("IntersectionObserver", IntersectionObserverStub);
    Element.prototype.scrollIntoView = vi.fn();
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
      font: "",
      measureText: () => ({ width: 0 }),
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
    history.replaceState(null, "", "/");
    tauriApi.listProjects.mockResolvedValue([project]);
    tauriApi.touchProject.mockImplementation(async (_projectId, selection) => ({
      ...project,
      comparison: selection ?? project.comparison,
    }));
    tauriApi.saveProjectComparison.mockImplementation(
      async (_projectId, selection) => ({
        ...project,
        comparison: selection,
      }),
    );
    tauriApi.validateRepository.mockResolvedValue({
      repoPath: project.repoPath,
      suggestedName: project.name,
      detectedBaseRef: "main",
      currentBranch: "feature",
      recentBranches: ["main"],
      localBranches: ["feature", "main"],
      remoteBranches: ["origin/main"],
      recentCommits: [
        {
          sha: "1234567890abcdef",
          shortSha: "1234567",
          subject: "Example change",
        },
      ],
    });
    tauriApi.getUserPreferences.mockResolvedValue({
      ai: { outputLanguage: "english" },
      changeDetail: {
        changedFilesPanel: { open: true, width: 225 },
        aiPanel: { open: true, width: 290 },
        diff: { mode: "split", wrapLongLines: false },
      },
    });
    tauriApi.saveUserPreferences.mockImplementation(
      async (preferences) => preferences,
    );
    tauriApi.getDiffSummary.mockResolvedValue(summary);
    tauriApi.getFileDiffs.mockResolvedValue([fileDiff]);
    updater.check.mockResolvedValue(null);
    updater.downloadAndInstall.mockResolvedValue(undefined);
    updater.relaunch.mockResolvedValue(undefined);
    updater.onFocusChanged.mockResolvedValue(vi.fn());
    tauriApi.getChangeReviewAvailability.mockImplementation(
      async (
        _projectId: string,
        selection: { base: string; target: string },
      ) => ({
        available:
          (selection.base === "HEAD" && selection.target === ".") ||
          (selection.base === "main" &&
            ["HEAD", "feature"].includes(selection.target)),
        scopeLabel: `${selection.base === "HEAD" ? "feature" : selection.base} → ${selection.target === "." ? "working tree" : selection.target === "HEAD" ? "feature" : selection.target}`,
        reason: undefined,
      }),
    );
  });

  afterEach(() => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
    vi.unstubAllGlobals();
    Element.prototype.scrollIntoView = originalScrollIntoView;
    HTMLCanvasElement.prototype.getContext = originalGetContext;
  });

  it("reloads the open project when window focus returns", async () => {
    history.replaceState(null, "", "/?project=alpha&file=src%2Fexample.ts");
    render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );
    await waitFor(() =>
      expect(Element.prototype.scrollIntoView).toHaveBeenCalledTimes(2),
    );
    expect(screen.getByText("feature → working tree")).toBeInTheDocument();
    expect(
      screen.getByText("Scope: feature → working tree"),
    ).toBeInTheDocument();
    expect(screen.queryByText("HEAD → working tree")).not.toBeInTheDocument();
    vi.mocked(Element.prototype.scrollIntoView).mockClear();
    await fireEvent.focus(window);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2),
    );
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalledTimes(2));
    expect(tauriApi.validateRepository).toHaveBeenCalledTimes(2);

    expect(new URLSearchParams(location.search).get("file")).toBe(
      "src/example.ts",
    );
    expect(Element.prototype.scrollIntoView).not.toHaveBeenCalled();
  });

  it("returns to the default comparison when a selected branch was deleted", async () => {
    tauriApi.getDiffSummary.mockImplementation(
      async (
        _projectId: string,
        selection: { base: string; target: string },
      ) => ({
        ...summary,
        selection,
        comparison: {
          fromLabel: selection.base,
          toLabel: selection.target === "." ? "working tree" : selection.target,
          fromSha: summary.comparison.fromSha,
          toSha:
            selection.target === "." ? undefined : summary.comparison.fromSha,
        },
      }),
    );
    tauriApi.validateRepository
      .mockResolvedValueOnce({
        repoPath: project.repoPath,
        suggestedName: project.name,
        detectedBaseRef: "main",
        currentBranch: "feature",
        recentBranches: ["main"],
        localBranches: ["feature", "main"],
        remoteBranches: ["origin/main"],
        recentCommits: [],
      })
      .mockResolvedValueOnce({
        repoPath: project.repoPath,
        suggestedName: project.name,
        detectedBaseRef: null,
        currentBranch: "feature",
        recentBranches: [],
        localBranches: ["feature"],
        remoteBranches: ["origin/main"],
        recentCommits: [],
      });
    history.replaceState(null, "", "/?project=alpha&base=main&target=HEAD");
    render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
        base: "main",
        target: "HEAD",
      }),
    );
    await waitFor(() =>
      expect(screen.queryByText("Loading changes…")).not.toBeInTheDocument(),
    );

    await fireEvent.focus(window);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
        base: "HEAD",
        target: ".",
      }),
    );
    await waitFor(() => {
      expect(new URLSearchParams(location.search).has("base")).toBe(false);
      expect(new URLSearchParams(location.search).has("target")).toBe(false);
    });
    expect(tauriApi.saveProjectComparison).toHaveBeenCalledWith("alpha", {
      base: "HEAD",
      target: ".",
    });
    expect(
      screen.getByRole("button", {
        name: "Change comparison. Current: feature → working tree",
      }),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(tauriApi.getFileDiffs).toHaveBeenLastCalledWith(
        "alpha",
        { base: "HEAD", target: "." },
        ["src/example.ts"],
      ),
    );
  });

  it("does not reload from the project list", async () => {
    render(Page);
    await waitFor(() => expect(tauriApi.listProjects).toHaveBeenCalledOnce());

    await fireEvent.focus(window);

    expect(tauriApi.getDiffSummary).not.toHaveBeenCalled();
  });

  it("shows each project's saved comparison on the project list", async () => {
    tauriApi.listProjects.mockResolvedValue([
      {
        ...project,
        comparison: { base: "main", target: "feature" },
      },
    ]);

    render(Page);

    expect(await screen.findByText("main → feature")).toBeInTheDocument();
    expect(
      screen.queryByText("Current branch → working tree"),
    ).not.toBeInTheDocument();
  });

  it("restores the project's saved comparison when it is opened", async () => {
    const savedProject = {
      ...project,
      comparison: { base: "main", target: "feature" },
    };
    tauriApi.listProjects.mockResolvedValue([savedProject]);
    tauriApi.touchProject.mockResolvedValue(savedProject);
    history.replaceState(null, "", "/?project=alpha");

    render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledWith("alpha", {
        base: "main",
        target: "feature",
      }),
    );
    await waitFor(() =>
      expect(tauriApi.getFileDiffs).toHaveBeenCalledWith(
        "alpha",
        { base: "main", target: "feature" },
        ["src/example.ts"],
      ),
    );
  });

  it("uses the fallback comparison returned when a saved ref no longer exists", async () => {
    tauriApi.listProjects.mockResolvedValue([
      {
        ...project,
        comparison: { base: "deleted-branch", target: "HEAD" },
      },
    ]);
    tauriApi.touchProject.mockResolvedValue(project);
    history.replaceState(null, "", "/?project=alpha");

    render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledWith("alpha", {
        base: "HEAD",
        target: ".",
      }),
    );
    await waitFor(() =>
      expect(tauriApi.getFileDiffs).toHaveBeenCalledWith(
        "alpha",
        { base: "HEAD", target: "." },
        ["src/example.ts"],
      ),
    );
  });

  it("keeps the rendered diff visible while an automatic refresh is pending", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument(),
    );

    let finishRefresh: (diffs: FileDiff[]) => void = () => {};
    tauriApi.getFileDiffs.mockImplementationOnce(
      () =>
        new Promise<FileDiff[]>((resolve) => {
          finishRefresh = resolve;
        }),
    );

    await fireEvent.focus(window);
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalledTimes(2));

    expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument();
    finishRefresh([fileDiff]);
    await waitFor(() =>
      expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument(),
    );
  });

  it("orders the diff feed to match the changed-files tree", async () => {
    tauriApi.getDiffSummary.mockResolvedValue({
      ...summary,
      files: [
        {
          newPath: "CHANGELOG.md",
          status: "added",
          additions: 1,
          deletions: 0,
        },
        {
          newPath: "docs/overview.md",
          status: "modified",
          additions: 1,
          deletions: 1,
        },
      ],
    });
    tauriApi.getFileDiffs.mockResolvedValue([]);
    history.replaceState(null, "", "/?project=alpha");

    const { container } = render(Page);

    await waitFor(() =>
      expect(container.querySelectorAll(".file-diff")).toHaveLength(2),
    );
    const treePaths = [
      ...container.querySelectorAll<HTMLElement>(".file-row"),
    ].map((row) => row.title);
    const feedPaths = [
      ...container.querySelectorAll<HTMLElement>(".file-diff"),
    ].map((section) => section.dataset.diffPath);

    expect(treePaths).toEqual(["docs/overview.md", "CHANGELOG.md"]);
    expect(feedPaths).toEqual(treePaths);
  });

  it("keeps diff controls in the top bar and change totals in the file sidebar", async () => {
    history.replaceState(null, "", "/?project=alpha");
    const { container } = render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    const topbarControls = container.querySelector(".topbar .view-controls");
    expect(topbarControls).toContainElement(screen.getByTitle("Split diff"));
    expect(topbarControls).toContainElement(screen.getByTitle("Unified diff"));
    expect(topbarControls).toContainElement(
      screen.getByTitle("Wrap long lines"),
    );
    expect(container.querySelector(".content-toolbar")).not.toBeInTheDocument();
    expect(
      await screen.findByLabelText("1 changed file, 2 additions, 1 deletion"),
    ).toBeInTheDocument();
    expect(screen.queryByText("12345678")).not.toBeInTheDocument();
  });

  it("shows an available update beside settings on the project list", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    updater.check.mockResolvedValue({
      downloadAndInstall: updater.downloadAndInstall,
    });
    const { container } = render(Page);

    const installButton = await within(container).findByRole("button", {
      name: "Install available update",
    });
    expect(
      container.querySelector(".home header .app-actions"),
    ).toContainElement(installButton);
    expect(
      container.querySelector(".home header .app-actions")?.lastElementChild,
    ).toBe(installButton);
  });

  it("shows an available update in the workspace app actions", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    updater.check.mockResolvedValue({
      downloadAndInstall: updater.downloadAndInstall,
    });
    history.replaceState(null, "", "/?project=alpha");
    const { container } = render(Page);

    await waitFor(() => expect(tauriApi.getDiffSummary).toHaveBeenCalledOnce());
    const topbar = container.querySelector<HTMLElement>(".topbar");
    const installButton = within(container).getByRole("button", {
      name: "Install available update",
    });
    expect(topbar?.querySelector(".top-actions")).not.toContainElement(
      installButton,
    );
    expect(topbar?.lastElementChild).toBe(installButton);
  });

  it("coalesces repeated focus events", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );
    await waitFor(() =>
      expect(screen.queryByText("Loading changes…")).not.toBeInTheDocument(),
    );
    await waitFor(() =>
      expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument(),
    );
    const initialCalls = tauriApi.getDiffSummary.mock.calls.length;

    await fireEvent.focus(window);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(initialCalls + 1),
    );
    await fireEvent.focus(window);

    expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(initialCalls + 1);
  });

  it("applies a branch comparison and stores it in the URL", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    expect(
      screen.queryByRole("combobox", { name: "From" }),
    ).not.toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", {
        name: "Change comparison. Current: feature → working tree",
      }),
    );
    expect(
      screen.getByRole("dialog", { name: "Change comparison" }),
    ).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("combobox", { name: "From" }));
    let menu = screen.getByRole("listbox", { name: "From revisions" });
    await fireEvent.click(within(menu).getByRole("option", { name: "main" }));
    await fireEvent.click(screen.getByRole("combobox", { name: "To" }));
    menu = screen.getByRole("listbox", { name: "To revisions" });
    const localBranches = within(menu).getByRole("region", {
      name: "Local branches",
    });
    await fireEvent.click(
      within(localBranches).getByRole("button", { name: /Local branches/ }),
    );
    await fireEvent.click(
      within(localBranches).getByRole("option", { name: "feature" }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
        base: "main",
        target: "feature",
      }),
    );
    expect(tauriApi.saveProjectComparison).toHaveBeenCalledWith("alpha", {
      base: "main",
      target: "feature",
    });
    expect(new URLSearchParams(location.search).get("base")).toBe("main");
    expect(new URLSearchParams(location.search).get("target")).toBe("feature");
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Change comparison" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("closes the comparison dialog while the selected changes load", async () => {
    let finishLoading: (value: DiffSummary) => void = () => {};
    tauriApi.getDiffSummary
      .mockResolvedValueOnce(summary)
      .mockImplementationOnce(
        () =>
          new Promise<DiffSummary>((resolve) => {
            finishLoading = resolve;
          }),
      );
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(
      screen.getByRole("button", {
        name: "Change comparison. Current: feature → working tree",
      }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Compare main → feature" }),
    );

    expect(
      screen.queryByRole("dialog", { name: "Change comparison" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Loading changes…")).toBeInTheDocument();

    finishLoading({
      ...summary,
      selection: { base: "main", target: "HEAD" },
    });
    await waitFor(() =>
      expect(screen.queryByText("Loading changes…")).not.toBeInTheDocument(),
    );
  });

  it("changes comparison during loading and ignores the stale response", async () => {
    let finishInitial: (value: DiffSummary) => void = () => {};
    let finishComparison: (value: DiffSummary) => void = () => {};
    tauriApi.getDiffSummary
      .mockImplementationOnce(
        () =>
          new Promise<DiffSummary>((resolve) => {
            finishInitial = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<DiffSummary>((resolve) => {
            finishComparison = resolve;
          }),
      );
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(
      screen.getByRole("button", {
        name: "Change comparison. Current: feature → working tree",
      }),
    );
    const compare = screen.getByRole("button", {
      name: "Compare main → feature",
    });
    expect(compare).toBeEnabled();
    await fireEvent.click(compare);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2),
    );

    finishInitial(summary);
    await Promise.resolve();
    expect(screen.getByText("Loading changes…")).toBeInTheDocument();

    finishComparison({
      ...summary,
      selection: { base: "main", target: "HEAD" },
      comparison: {
        ...summary.comparison,
        fromLabel: "main",
        toLabel: "HEAD",
        toSha: summary.comparison.fromSha,
      },
    });
    await waitFor(() =>
      expect(screen.queryByText("Loading changes…")).not.toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", {
        name: "Change comparison. Current: main → feature",
      }),
    ).toBeInTheDocument();
  });

  it("applies quick comparisons using the configured base and current branch", async () => {
    tauriApi.getDiffSummary.mockImplementation(
      async (
        _projectId: string,
        selection: { base: string; target: string },
      ) => ({
        ...summary,
        selection,
        comparison: {
          fromLabel: selection.base,
          toLabel: selection.target === "." ? "working tree" : selection.target,
          fromSha: summary.comparison.fromSha,
          toSha:
            selection.target === "." ? undefined : summary.comparison.fromSha,
        },
      }),
    );
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(
      screen.getByRole("button", {
        name: "Change comparison. Current: feature → working tree",
      }),
    );
    const dialog = screen.getByRole("dialog", { name: "Change comparison" });
    const comparisonForm = within(dialog).getByRole("form", {
      name: "Choose changes to review",
    });
    const quickComparisons = within(dialog).getByRole("region", {
      name: "Quick comparisons",
    });
    expect(
      comparisonForm.compareDocumentPosition(quickComparisons) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      within(quickComparisons).getByRole("status", {
        name: "Current comparison",
      }),
    ).toHaveTextContent("Current");
    expect(
      within(quickComparisons).queryByRole("button", {
        name: "Compare feature → Working tree",
      }),
    ).not.toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: /main → feature/ }),
    );

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
        base: "main",
        target: "HEAD",
      }),
    );
    expect(new URLSearchParams(location.search).get("base")).toBe("main");
    expect(new URLSearchParams(location.search).get("target")).toBe("HEAD");
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Change comparison" }),
      ).not.toBeInTheDocument(),
    );

    await fireEvent.click(
      screen.getByRole("button", {
        name: "Change comparison. Current: main → feature",
      }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: /feature → Working tree/ }),
    );

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
        base: "HEAD",
        target: ".",
      }),
    );
    expect(new URLSearchParams(location.search).has("base")).toBe(false);
    expect(new URLSearchParams(location.search).has("target")).toBe(false);
  });

  it("disables Change Review and explains an unsupported comparison", async () => {
    tauriApi.getChangeReviewAvailability.mockResolvedValue({
      available: false,
      scopeLabel: "main → working tree",
      reason:
        "Change Review supports the working tree only when the comparison starts at HEAD.",
    });
    history.replaceState(null, "", "/?project=alpha");
    render(Page);

    const reason = await screen.findByText(
      "Change Review supports the working tree only when the comparison starts at HEAD.",
    );
    expect(reason).toBeInTheDocument();
    expect(
      screen.getByText(
        "Codex reviews the selected changes and highlights potential issues.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Scope: main → working tree")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run review" })).toBeDisabled();
  });

  it("shows the Change Review loading state only in the action button", async () => {
    tauriApi.runChangeReview.mockReturnValue(new Promise(() => {}));
    history.replaceState(null, "", "/?project=alpha");
    render(Page);

    const runReview = await screen.findByRole("button", { name: "Run review" });
    await waitFor(() => expect(runReview).toBeEnabled());
    await fireEvent.click(runReview);

    expect(
      await screen.findByRole("button", { name: "Reviewing…" }),
    ).toBeDisabled();
    expect(
      screen.queryByText(
        "Codex is reviewing this comparison. Large changes may take a while…",
      ),
    ).not.toBeInTheDocument();
  });

  it("renders file explanations next to the file and Change Review in the side panel", async () => {
    tauriApi.explainFileChange.mockResolvedValue({
      summary: "This file now builds review context before returning evidence.",
      inferredIntent: "Make the changed review flow easier to verify.",
      keyChanges: ["Build context asynchronously."],
      references: [
        { path: "src/example.ts", startLine: 1, endLine: 1, side: "new" },
      ],
      caveats: [],
    });
    tauriApi.runChangeReview.mockResolvedValue({
      summary: "The comparison adds evidence-aware review output.",
      inferredIntent: "Improve review traceability.",
      groups: [
        {
          id: "review-output",
          title: "Review output",
          summary: "Returns evidence with the review.",
          files: ["src/example.ts"],
          keyPoints: ["Evidence is explicit."],
        },
      ],
      findings: [],
      caveats: [],
    });
    history.replaceState(null, "", "/?project=alpha");
    render(Page);

    await fireEvent.click(
      await screen.findByRole("button", {
        name: "Explain changes in src/example.ts",
      }),
    );
    expect(
      await screen.findByText(
        "This file now builds review context before returning evidence.",
      ),
    ).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Run review" }));
    expect(
      await screen.findByText(
        "The comparison adds evidence-aware review output.",
      ),
    ).toBeInTheDocument();
    expect(tauriApi.runChangeReview).toHaveBeenCalledWith("alpha", {
      base: "HEAD",
      target: ".",
    });
    expect(notifyReviewComplete).toHaveBeenCalledWith("file");
    expect(notifyReviewComplete).toHaveBeenCalledWith("change");

    await fireEvent.focus(window);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2),
    );
    expect(
      screen.getByText(
        "This file now builds review context before returning evidence.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("The comparison adds evidence-aware review output."),
    ).toBeInTheDocument();
  });

  it("resizes both side panels from their drag handles", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    const sidebarHandle = screen.getByRole("separator", {
      name: "Resize changed files sidebar",
    });
    const aiPanelHandle = screen.getByRole("separator", {
      name: "Resize AI panel",
    });
    const workspace = sidebarHandle.parentElement;
    expect(workspace).not.toBeNull();
    vi.spyOn(workspace as HTMLElement, "getBoundingClientRect").mockReturnValue(
      {
        width: 1200,
      } as DOMRect,
    );

    const initialSidebarWidth = Number(
      sidebarHandle.getAttribute("aria-valuenow"),
    );
    await fireEvent.pointerDown(sidebarHandle, { button: 0, clientX: 200 });
    await fireEvent.pointerMove(window, { clientX: 250 });
    await fireEvent.pointerUp(window);
    expect(sidebarHandle).toHaveAttribute(
      "aria-valuenow",
      String(initialSidebarWidth + 50),
    );

    const initialAiPanelWidth = Number(
      aiPanelHandle.getAttribute("aria-valuenow"),
    );
    await fireEvent.pointerDown(aiPanelHandle, { button: 0, clientX: 800 });
    await fireEvent.pointerMove(window, { clientX: 750 });
    await fireEvent.pointerUp(window);
    expect(aiPanelHandle).toHaveAttribute(
      "aria-valuenow",
      String(initialAiPanelWidth + 50),
    );
  });

  it("supports keyboard resizing and hides handles with their panels", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    const sidebarHandle = screen.getByRole("separator", {
      name: "Resize changed files sidebar",
    });
    const workspace = sidebarHandle.parentElement;
    expect(workspace).not.toBeNull();
    vi.spyOn(workspace as HTMLElement, "getBoundingClientRect").mockReturnValue(
      {
        width: 1200,
      } as DOMRect,
    );

    const initialWidth = Number(sidebarHandle.getAttribute("aria-valuenow"));
    await fireEvent.keyDown(sidebarHandle, { key: "ArrowRight" });
    expect(sidebarHandle).toHaveAttribute(
      "aria-valuenow",
      String(initialWidth + 10),
    );

    await fireEvent.click(
      screen.getByRole("button", { name: "Hide changed files sidebar" }),
    );
    expect(
      screen.queryByRole("separator", { name: "Resize changed files sidebar" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("separator", { name: "Resize AI panel" }),
    ).toBeInTheDocument();
  });

  it("restores change detail preferences", async () => {
    tauriApi.getUserPreferences.mockResolvedValue({
      ai: { outputLanguage: "japanese" },
      changeDetail: {
        changedFilesPanel: { open: false, width: 310 },
        aiPanel: { open: true, width: 360 },
        diff: { mode: "unified", wrapLongLines: true },
      },
    });
    history.replaceState(null, "", "/?project=alpha");

    render(Page);

    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );
    expect(
      screen.getByRole("button", { name: "Show changed files sidebar" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("separator", { name: "Resize changed files sidebar" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("separator", { name: "Resize AI panel" }),
    ).toHaveAttribute("aria-valuenow", "360");
    expect(screen.getByTitle("Unified diff")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByTitle("Wrap long lines")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(
      screen.queryByRole("combobox", { name: "Review output language" }),
    ).not.toBeInTheDocument();

    await fireEvent.click(screen.getByTitle("AI settings"));
    expect(
      screen.getByRole("combobox", { name: "Review output language" }),
    ).toHaveTextContent("日本語");
    expect(tauriApi.saveUserPreferences).not.toHaveBeenCalled();
  });

  it("saves change detail preferences after controls change", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(screen.getByTitle("Unified diff"));
    await fireEvent.click(screen.getByTitle("Wrap long lines"));
    await fireEvent.click(
      screen.getByRole("button", { name: "Hide changed files sidebar" }),
    );
    await fireEvent.click(screen.getByTitle("Toggle AI panel"));

    await waitFor(() =>
      expect(tauriApi.saveUserPreferences).toHaveBeenLastCalledWith({
        ai: { outputLanguage: "english" },
        changeDetail: {
          changedFilesPanel: { open: false, width: 225 },
          aiPanel: { open: false, width: 290 },
          diff: { mode: "unified", wrapLongLines: true },
        },
      }),
    );
  });

  it("saves the review output language", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(screen.getByTitle("AI settings"));
    await fireEvent.click(
      screen.getByRole("combobox", { name: "Review output language" }),
    );
    await fireEvent.click(
      within(
        screen.getByRole("listbox", { name: "Review output language options" }),
      ).getByRole("option", { name: "日本語" }),
    );

    await waitFor(() =>
      expect(tauriApi.saveUserPreferences).toHaveBeenLastCalledWith({
        ai: { outputLanguage: "japanese" },
        changeDetail: {
          changedFilesPanel: { open: true, width: 225 },
          aiPanel: { open: true, width: 290 },
          diff: { mode: "split", wrapLongLines: false },
        },
      }),
    );
  });

  it("supports application keyboard shortcuts", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.keyDown(window, { key: "b", code: "KeyB", metaKey: true });
    expect(
      screen.getByRole("button", { name: "Show changed files sidebar" }),
    ).toBeInTheDocument();

    await fireEvent.keyDown(window, {
      key: "∫",
      code: "KeyB",
      metaKey: true,
      altKey: true,
    });
    expect(
      screen.queryByRole("separator", { name: "Resize AI panel" }),
    ).not.toBeInTheDocument();

    await fireEvent.keyDown(window, { key: ",", code: "Comma", metaKey: true });
    expect(
      screen.getByRole("dialog", { name: "AI settings" }),
    ).toBeInTheDocument();

    await fireEvent.keyDown(window, { key: "Escape" });
    expect(
      screen.queryByRole("dialog", { name: "AI settings" }),
    ).not.toBeInTheDocument();

    await waitFor(() =>
      expect(tauriApi.saveUserPreferences).toHaveBeenLastCalledWith({
        ai: { outputLanguage: "english" },
        changeDetail: {
          changedFilesPanel: { open: false, width: 225 },
          aiPanel: { open: false, width: 290 },
          diff: { mode: "split", wrapLongLines: false },
        },
      }),
    );
  });

  it("opens project settings from the project switcher", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.click(
      screen.getByRole("button", { name: "Switch project. Current: Alpha" }),
    );
    await fireEvent.click(
      screen.getByRole("menuitem", { name: "Project settings" }),
    );

    await waitFor(() =>
      expect(
        screen.getByRole("dialog", { name: "Project settings" }),
      ).toBeInTheDocument(),
    );
  });

  it("refreshes the change details with Cmd+R", async () => {
    history.replaceState(null, "", "/?project=alpha&file=src%2Fexample.ts");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    const event = new KeyboardEvent("keydown", {
      key: "r",
      code: "KeyR",
      metaKey: true,
      cancelable: true,
    });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2),
    );
    expect(tauriApi.getDiffSummary).toHaveBeenLastCalledWith("alpha", {
      base: "HEAD",
      target: ".",
    });
    expect(new URLSearchParams(location.search).get("file")).toBe(
      "src/example.ts",
    );
  });

  it("finds diff text with Cmd+F without filtering changed files", async () => {
    history.replaceState(null, "", "/?project=alpha&file=src%2Fexample.ts");
    render(Page);
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalled());

    const shortcut = new KeyboardEvent("keydown", {
      key: "f",
      code: "KeyF",
      metaKey: true,
      cancelable: true,
    });
    window.dispatchEvent(shortcut);

    expect(shortcut.defaultPrevented).toBe(true);
    const input = await screen.findByRole("searchbox", {
      name: "Find in changes",
    });
    await waitFor(() => expect(input).toHaveFocus());

    await fireEvent.input(input, { target: { value: "ANSWER" } });
    await waitFor(() => expect(screen.getByText("0 / 2")).toBeInTheDocument());
    expect(screen.getByText("example.ts")).toBeInTheDocument();
    expect(screen.getByTitle("2 matches")).toBeInTheDocument();

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByText("1 / 2")).toBeInTheDocument();

    await fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(screen.getByText("2 / 2")).toBeInTheDocument();

    await fireEvent.keyDown(input, { key: "Escape" });
    expect(
      screen.queryByRole("searchbox", { name: "Find in changes" }),
    ).not.toBeInTheDocument();
  });

  it("leaves Cmd+R to the browser from the project list", async () => {
    render(Page);
    await waitFor(() => expect(tauriApi.listProjects).toHaveBeenCalledOnce());

    const event = new KeyboardEvent("keydown", {
      key: "r",
      code: "KeyR",
      metaKey: true,
      cancelable: true,
    });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    expect(tauriApi.getDiffSummary).not.toHaveBeenCalled();
  });

  it("opens AI settings from the project list and closes it with Escape", async () => {
    render(Page);
    await waitFor(() => expect(tauriApi.listProjects).toHaveBeenCalledOnce());

    await fireEvent.keyDown(window, { key: ",", code: "Comma", metaKey: true });
    expect(
      screen.getByRole("dialog", { name: "AI settings" }),
    ).toBeInTheDocument();

    await fireEvent.keyDown(window, { key: "Escape" });
    expect(
      screen.queryByRole("dialog", { name: "AI settings" }),
    ).not.toBeInTheDocument();
  });

  it("ignores repeated or modified shortcut keydown events", async () => {
    history.replaceState(null, "", "/?project=alpha");
    render(Page);
    await waitFor(() =>
      expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1),
    );

    await fireEvent.keyDown(window, {
      key: "b",
      code: "KeyB",
      metaKey: true,
      repeat: true,
    });
    await fireEvent.keyDown(window, {
      key: "B",
      code: "KeyB",
      metaKey: true,
      shiftKey: true,
    });

    expect(
      screen.getByRole("button", { name: "Hide changed files sidebar" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("separator", { name: "Resize AI panel" }),
    ).toBeInTheDocument();
    expect(tauriApi.saveUserPreferences).not.toHaveBeenCalled();
  });
});
