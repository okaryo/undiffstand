import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { UpdateState } from "$lib/domain/update";
import UpdateAction from "./UpdateAction.svelte";

function renderUpdateAction(state: UpdateState, onInstall = vi.fn()) {
  const baseElement = document.createElement("div");
  document.body.append(baseElement);
  return {
    ...render(UpdateAction, { state, onInstall }, { baseElement }),
    onInstall,
  };
}

describe("UpdateAction", () => {
  it("stays hidden when no update is available", () => {
    const { container } = renderUpdateAction("idle");

    expect(container.querySelector("button")).not.toBeInTheDocument();
  });

  it("offers installation when an update is available", async () => {
    const { container, onInstall } = renderUpdateAction("available");
    const button = container.querySelector("button");

    expect(button).toHaveAccessibleName("Install available update");
    expect(button).toHaveTextContent("Install update");
    await fireEvent.click(button!);

    expect(onInstall).toHaveBeenCalledOnce();
  });

  it("disables the action while installation is running", () => {
    const { container } = renderUpdateAction("installing");
    const button = container.querySelector("button");

    expect(button).toHaveAccessibleName("Installing update");
    expect(button).toHaveTextContent("Installing…");
    expect(button).toBeDisabled();
  });
});
