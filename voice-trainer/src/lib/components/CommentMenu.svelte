<script lang="ts">
  // A "⋯" button that opens a small popup with a Delete action.
  let { onDelete }: { onDelete: () => void } = $props();

  let open = $state(false);

  function toggle() {
    open = !open;
  }

  function del(e: MouseEvent) {
    e.stopPropagation();
    open = false;
    onDelete();
  }

  // Close when clicking anywhere else. The listener is attached *after* the
  // opening click has finished propagating, so it doesn't immediately re-close.
  $effect(() => {
    if (!open) return;
    const close = () => (open = false);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  });
</script>

<span class="menu-wrap">
  <button class="dots" aria-label="Comment options" aria-haspopup="menu" aria-expanded={open} onclick={toggle}>
    ⋯
  </button>
  {#if open}
    <div class="popup" role="menu">
      <button class="popup-item delete" role="menuitem" onclick={del}>Delete</button>
    </div>
  {/if}
</span>

<style>
  .menu-wrap { position: relative; display: inline-flex; }
  .dots {
    background: transparent;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }
  .dots:hover { color: var(--text); background: rgba(255,255,255,0.07); }

  .popup {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 30;
    background: #1a1a3a;
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.45);
    padding: 0.25rem;
    min-width: 110px;
  }
  .popup-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    font-size: 0.8rem;
    padding: 0.4rem 0.6rem;
    border-radius: 5px;
    cursor: pointer;
  }
  .popup-item.delete { color: #e74c6f; }
  .popup-item.delete:hover { background: rgba(231,76,111,0.14); }
</style>
