import type {
  ChangeReviewAvailability,
  ChangeReviewReport,
  DiffExplanation,
  InlineAnswer
} from '$lib/domain/ai';
import type { DiffSelection } from '$lib/domain/diff';
import { normalizeError } from '$lib/domain/error';
import { notifyReviewComplete } from '$lib/services/notification';
import type { AppApi } from '$lib/services/api';

export class AiReviewController {
  fileExplanations = $state<Record<string, DiffExplanation | undefined>>({});
  fileLoading = $state<Record<string, boolean | undefined>>({});
  fileErrors = $state<Record<string, string | undefined>>({});
  report = $state<ChangeReviewReport>();
  loading = $state(false);

  private generation = 0;

  constructor(
    private readonly api: Pick<
      AppApi,
      'explainFileChange' | 'askInlineQuestion' | 'runChangeReview'
    >,
    private readonly onError: (error: unknown) => void
  ) {}

  reset() {
    this.generation += 1;
    this.loading = false;
    this.fileExplanations = {};
    this.fileLoading = {};
    this.fileErrors = {};
    this.report = undefined;
  }

  async explainFile(projectId: string, selection: DiffSelection, path: string) {
    if (this.fileLoading[path]) return;
    const generation = this.generation;
    this.fileLoading[path] = true;
    this.fileErrors[path] = undefined;
    this.fileExplanations[path] = undefined;
    try {
      const explanation = await this.api.explainFileChange(projectId, { ...selection }, path);
      if (generation !== this.generation) return;
      this.fileExplanations[path] = explanation;
      void notifyReviewComplete('file');
    } catch (error) {
      if (generation !== this.generation) return;
      this.fileErrors[path] = normalizeError(error).message;
    } finally {
      if (generation === this.generation) this.fileLoading[path] = false;
    }
  }

  async askInline(
    projectId: string,
    selection: DiffSelection,
    path: string,
    side: 'old' | 'new',
    startLine: number,
    endLine: number,
    question: string
  ): Promise<InlineAnswer> {
    const generation = this.generation;
    const answer = await this.api.askInlineQuestion(
      projectId,
      { ...selection },
      {
        path,
        side,
        startLine,
        endLine,
        question
      }
    );
    if (generation !== this.generation) {
      throw new Error('The comparison changed before Codex answered.');
    }
    void notifyReviewComplete('inline');
    return answer;
  }

  async review(
    projectId: string,
    selection: DiffSelection,
    availability?: ChangeReviewAvailability
  ) {
    if (!availability?.available || this.loading) return;
    const generation = this.generation;
    this.loading = true;
    this.report = undefined;
    this.onError(null);
    try {
      const report = await this.api.runChangeReview(projectId, { ...selection });
      if (generation !== this.generation) return;
      this.report = report;
      void notifyReviewComplete('change');
    } catch (error) {
      if (generation !== this.generation) return;
      this.onError(error);
    } finally {
      if (generation === this.generation) this.loading = false;
    }
  }
}
