<script lang="ts">
  import { onDestroy } from "svelte";

  let {
    label,
    value,
    minimum,
    maximum,
    direction = 1,
    cssProperty,
    constrain,
    onChange,
    onCommit,
    onStart = () => {},
    onReset,
  }: {
    label: string;
    value: number;
    minimum: number;
    maximum: number;
    direction?: 1 | -1;
    cssProperty: string;
    constrain: (requestedWidth: number, workspaceWidth: number) => number;
    onChange: (width: number) => void;
    onCommit: () => void;
    onStart?: () => void;
    onReset: () => void;
  } = $props();

  let active = $state<
    | {
        startX: number;
        startWidth: number;
        currentWidth: number;
        workspaceWidth: number;
        workspace: HTMLElement;
      }
    | undefined
  >();
  let pendingClientX: number | undefined;
  let animationFrame: number | undefined;
  let previousCursor = "";
  let previousUserSelect = "";

  onDestroy(stopResize);

  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    const workspace = (event.currentTarget as HTMLElement).parentElement;
    if (!workspace) return;

    event.preventDefault();
    active = {
      startX: event.clientX,
      startWidth: value,
      currentWidth: value,
      workspaceWidth: workspace.getBoundingClientRect().width,
      workspace,
    };
    previousCursor = document.body.style.cursor;
    previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", resize);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  function resize(event: PointerEvent) {
    if (!active) return;
    onStart();
    pendingClientX = event.clientX;
    if (animationFrame !== undefined) return;
    animationFrame = requestAnimationFrame(flushResize);
  }

  function flushResize() {
    animationFrame = undefined;
    if (!active || pendingClientX === undefined) return;
    const requestedWidth =
      active.startWidth + (pendingClientX - active.startX) * direction;
    pendingClientX = undefined;
    active.currentWidth = constrain(requestedWidth, active.workspaceWidth);
    active.workspace.style.setProperty(cssProperty, `${active.currentWidth}px`);
  }

  function stopResize() {
    if (!active) return;
    if (animationFrame !== undefined) {
      cancelAnimationFrame(animationFrame);
      animationFrame = undefined;
    }
    flushResize();
    onChange(active.currentWidth);
    onCommit();
    active = undefined;
    pendingClientX = undefined;
    document.body.style.cursor = previousCursor;
    document.body.style.userSelect = previousUserSelect;
    window.removeEventListener("pointermove", resize);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  function resizeWithKeyboard(event: KeyboardEvent) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const workspace = (event.currentTarget as HTMLElement).parentElement;
    if (!workspace) return;

    event.preventDefault();
    const workspaceWidth = workspace.getBoundingClientRect().width;
    const movement = event.key === "ArrowRight" ? 10 : -10;
    const requestedWidth =
      event.key === "Home"
        ? Number.NEGATIVE_INFINITY
        : event.key === "End"
          ? Number.POSITIVE_INFINITY
          : value + movement * direction;
    onChange(constrain(requestedWidth, workspaceWidth));
    onCommit();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex (separator is an interactive window splitter) -->
<div
  class="handle"
  class:resizing={active}
  role="separator"
  aria-label={label}
  aria-orientation="vertical"
  aria-valuemin={minimum}
  aria-valuemax={maximum}
  aria-valuenow={value}
  tabindex="0"
  onpointerdown={startResize}
  onkeydown={resizeWithKeyboard}
  ondblclick={onReset}
></div>

<style>
  .handle {
    position: relative;
    z-index: 2;
    min-width: var(--panel-handle-width);
    padding: 0;
    cursor: col-resize;
    touch-action: none;
    background: transparent;
    border: 0;
    border-radius: 0;
  }
  .handle::after {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    background: transparent;
    content: "";
    transform: translateX(-50%);
    transition: background 120ms ease;
  }
  .handle:hover::after,
  .handle:focus-visible::after,
  .handle.resizing::after {
    background: var(--accent-bright);
  }
  .handle:focus-visible {
    outline: 1px solid var(--accent-bright);
    outline-offset: -1px;
  }
</style>
