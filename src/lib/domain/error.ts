export type AppErrorCode =
  | "NOT_A_GIT_REPOSITORY"
  | "GIT_NOT_FOUND"
  | "INVALID_BASE_REF"
  | "NO_MERGE_BASE"
  | "CODEX_NOT_FOUND"
  | "CODEX_EXEC_FAILED"
  | "CODEX_RESPONSE_INVALID"
  | "UNKNOWN"
  | string;

export type AppError = {
  code: AppErrorCode;
  message: string;
  detail?: string;
};

export function normalizeError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const candidate = error as Partial<AppError>;
    return {
      code: typeof candidate.code === "string" ? candidate.code : "UNKNOWN",
      message:
        typeof candidate.message === "string"
          ? candidate.message
          : "Unexpected error.",
      detail:
        typeof candidate.detail === "string" ? candidate.detail : undefined,
    };
  }
  if (typeof error === "string") {
    try {
      return normalizeError(JSON.parse(error));
    } catch {
      return { code: "UNKNOWN", message: error };
    }
  }
  return { code: "UNKNOWN", message: "Unexpected error." };
}
