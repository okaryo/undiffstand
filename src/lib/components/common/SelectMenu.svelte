<script lang="ts">
  import { Check, ChevronDown } from '@lucide/svelte';

  let {
    id,
    value,
    label,
    options,
    disabled = false,
    fluid = false,
    onChange
  }: {
    id: string;
    value: string;
    label: string;
    options: { value: string; label: string; disabled?: boolean }[];
    disabled?: boolean;
    fluid?: boolean;
    onChange: (value: string) => void;
  } = $props();

  let open = $state(false);
  let activeIndex = $state(0);
  let trigger: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();

  let menuId = $derived(`${id}-options`);
  let selectedOption = $derived(options.find((option) => option.value === value));

  $effect(() => {
    if (disabled) open = false;
  });

  function selectedIndex() {
    const index = options.findIndex((option) => option.value === value && !option.disabled);
    return index >= 0 ? index : options.findIndex((option) => !option.disabled);
  }

  function openMenu() {
    if (disabled) return;
    activeIndex = selectedIndex();
    open = true;
  }

  function closeMenu({ restoreFocus = false } = {}) {
    open = false;
    if (restoreFocus) trigger.focus();
  }

  function select(optionValue: string) {
    const option = options.find((candidate) => candidate.value === optionValue);
    if (!option || option.disabled) return;
    onChange(optionValue);
    closeMenu({ restoreFocus: true });
  }

  function moveActive(direction: 1 | -1) {
    if (!options.some((option) => !option.disabled)) return;
    let next = activeIndex;
    do {
      next = (next + direction + options.length) % options.length;
    } while (options[next]?.disabled);
    activeIndex = next;
  }

  function handleTriggerKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) openMenu();
      else moveActive(event.key === 'ArrowDown' ? 1 : -1);
      return;
    }
    if (open && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) select(option.value);
    }
  }

  function handleWindowClick(event: MouseEvent) {
    if (
      open &&
      event.target instanceof Node &&
      !trigger.contains(event.target) &&
      !menu?.contains(event.target)
    ) {
      closeMenu();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      event.stopImmediatePropagation();
      closeMenu({ restoreFocus: true });
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="select-menu" class:fluid>
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    class:open
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={menuId}
    aria-activedescendant={open && activeIndex >= 0 ? `${id}-option-${activeIndex}` : undefined}
    {disabled}
    onclick={() => (open ? closeMenu() : openMenu())}
    onkeydown={handleTriggerKeydown}
  >
    <span>{selectedOption?.label ?? value}</span>
    <ChevronDown size={13} />
  </button>

  {#if open}
    <div bind:this={menu} id={menuId} class="menu" role="listbox" aria-label={`${label} options`}>
      {#each options as option, index (option.value)}
        <button
          id={`${id}-option-${index}`}
          type="button"
          role="option"
          aria-selected={value === option.value}
          class:active={value === option.value}
          class:highlighted={activeIndex === index}
          disabled={option.disabled}
          onmouseenter={() => (activeIndex = index)}
          onclick={() => select(option.value)}
        >
          <span>{option.label}</span>
          {#if value === option.value}<Check size={13} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select-menu {
    position: relative;
    width: 245px;
    max-width: 100%;
    min-width: 0;
  }

  .select-menu.fluid {
    flex: 1 1 auto;
    width: auto;
  }

  .trigger {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    width: 100%;
    height: 29px;
    gap: 7px;
    padding: 0 8px;
    color: #cbd3da;
    text-align: left;
    background: #0a1016;
    border: 1px solid #293541;
    border-radius: 6px;
    outline: none;
    font: 11px var(--mono);
    cursor: pointer;
  }

  .trigger span,
  .menu button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger.open,
  .trigger:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(87, 184, 142, 0.08);
  }

  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .menu {
    position: absolute;
    z-index: 70;
    top: calc(100% + 6px);
    left: 0;
    width: 100%;
    min-width: max-content;
    padding: 6px;
    background: #111923;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.42);
  }

  .menu button {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 15px;
    align-items: center;
    width: 100%;
    min-height: 29px;
    gap: 7px;
    padding: 5px 8px;
    color: #95a1ab;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 5px;
    font: 11px var(--mono);
    cursor: pointer;
  }

  .menu button:hover,
  .menu button:focus-visible,
  .menu button.highlighted,
  .menu button.active {
    color: var(--text);
    background: var(--hover);
    outline: none;
  }

  .menu button.active {
    background: rgba(87, 184, 142, 0.1);
  }

  .menu button:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
