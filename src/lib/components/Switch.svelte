<script lang="ts">
  let {
    checked,
    label,
    hint = "",
    disabled = false,
    onchange,
  }: {
    checked: boolean;
    label: string;
    hint?: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  } = $props();
</script>

<button
  type="button"
  class="switch"
  role="switch"
  aria-checked={checked}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="text">
    <span class="title">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</button>

<style>
  .switch {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    width: 100%;
    padding: 8px 0;
    text-align: left;
  }

  .switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .title {
    font-size: 0.875rem;
    font-weight: 500;
  }

  .hint {
    font-size: 0.75rem;
    line-height: 1.4;
    color: var(--ink-faint);
  }

  .track {
    flex: none;
    width: 38px;
    height: 22px;
    padding: 2px;
    border-radius: 11px;
    background: rgb(255 255 255 / 0.18);
    transition: background-color 160ms ease;
  }

  .thumb {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.35);
    transition: transform 200ms var(--ease-out);
  }

  .switch[aria-checked="true"] .track {
    background: var(--accent);
  }

  .switch[aria-checked="true"] .thumb {
    transform: translateX(16px);
  }
</style>
