import type {
  ChangeReviewAvailability,
  ChangeReviewReport,
  DiffExplanation,
  InlineAnswer,
  InlineQuestion,
} from "$lib/domain/ai";
import type { DiffSelection, DiffSummary, FileDiff } from "$lib/domain/diff";
import type { UserPreferences } from "$lib/domain/preferences";
import type {
  ProjectConfig,
  RepositoryInfo,
  SaveProjectInput,
} from "$lib/domain/project";

export interface AppApi {
  selectRepository(): Promise<string | null>;
  validateRepository(path: string): Promise<RepositoryInfo>;
  listProjects(): Promise<ProjectConfig[]>;
  saveProject(input: SaveProjectInput): Promise<ProjectConfig>;
  touchProject(
    projectId: string,
    selection?: DiffSelection,
  ): Promise<ProjectConfig>;
  saveProjectComparison(
    projectId: string,
    selection: DiffSelection,
  ): Promise<ProjectConfig>;
  removeProject(projectId: string): Promise<void>;
  getUserPreferences(): Promise<UserPreferences>;
  saveUserPreferences(preferences: UserPreferences): Promise<UserPreferences>;
  getDiffSummary(
    projectId: string,
    selection: DiffSelection,
  ): Promise<DiffSummary>;
  getFileDiffs(
    projectId: string,
    selection: DiffSelection,
    paths: string[],
  ): Promise<FileDiff[]>;
  explainFileChange(
    projectId: string,
    selection: DiffSelection,
    path: string,
  ): Promise<DiffExplanation>;
  askInlineQuestion(
    projectId: string,
    selection: DiffSelection,
    question: InlineQuestion,
  ): Promise<InlineAnswer>;
  getChangeReviewAvailability(
    projectId: string,
    selection: DiffSelection,
  ): Promise<ChangeReviewAvailability>;
  runChangeReview(
    projectId: string,
    selection: DiffSelection,
  ): Promise<ChangeReviewReport>;
}
