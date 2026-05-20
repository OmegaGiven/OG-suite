<script lang="ts" context="module">
  export type MobileSuiteNavItem = {
    id: string
    name: string
    disabled?: boolean
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte'

  export let navItems: MobileSuiteNavItem[] = []
  export let activeAppId = ''
  export let onSelectApp: (appId: string) => void = () => {}
  export let onOpenSettings: (() => void) | undefined = undefined
  export let title = 'Menu'
  export let align: 'inline' | 'right' = 'right'

  let open = false

  function selectApp(appId: string) {
    onSelectApp(appId)
    open = false
  }

  function openSettings() {
    onOpenSettings?.()
    open = false
  }
</script>

<div class:open class={`mobile-suite-menu mobile-suite-menu-${align}`}>
  <button
    class="mobile-suite-menu-trigger"
    aria-label="Open menu"
    aria-expanded={open}
    title="Menu"
    on:click={() => open = true}
  >
    <span aria-hidden="true"></span>
  </button>

  {#if open}
    <button class="mobile-suite-menu-backdrop" aria-label="Close menu" on:click={() => open = false}></button>
    <aside class="mobile-suite-menu-drawer" aria-label={title}>
      <header>
        <strong>{title}</strong>
        <button aria-label="Close menu" on:click={() => open = false}>
          <Icon name="collapse" size={18} />
        </button>
      </header>

      {#if navItems.length}
        <nav aria-label="Suite apps">
          {#each navItems as item}
            <button
              class:active={activeAppId === item.id}
              disabled={item.disabled}
              aria-current={activeAppId === item.id ? 'page' : undefined}
              on:click={() => selectApp(item.id)}
            >
              {item.name}
            </button>
          {/each}
        </nav>
      {/if}

      <div class="mobile-suite-menu-tools">
        <slot />
        {#if onOpenSettings}
          <button class="mobile-suite-menu-tool" on:click={openSettings}>
            <Icon name="settings" size={16} />
            <span>Settings</span>
          </button>
        {/if}
      </div>
    </aside>
  {/if}
</div>

<style>
  .mobile-suite-menu {
    display: none;
  }

  @media (max-width: 760px) {
    .mobile-suite-menu {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex: 0 0 auto;
    }

    .mobile-suite-menu.open {
      position: fixed;
      inset: 0;
      z-index: 2147483000;
      display: block;
      pointer-events: none;
    }

    :global(.feed-app:has(.mobile-suite-menu.open)),
    :global(.audio-app:has(.mobile-suite-menu.open)),
    :global(.files-app:has(.mobile-suite-menu.open)),
    :global(.suite-content:has(.mobile-suite-menu.open)),
    :global(.feed-hero:has(.mobile-suite-menu.open)),
    :global(.recorder-panel:has(.mobile-suite-menu.open)) {
      position: relative;
      z-index: 2147483000;
    }

    .mobile-suite-menu-right {
      margin-left: auto;
    }

    .mobile-suite-menu-trigger {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 34px;
      min-width: 34px;
      height: 34px;
      min-height: 34px;
      padding: 0;
      border: 1px solid var(--border, var(--og-border));
      border-radius: var(--field-radius, var(--og-field-radius));
      background: var(--surface-subtle, var(--og-surface-subtle));
      color: var(--text, var(--og-text));
    }

    .mobile-suite-menu-trigger span {
      display: block;
      position: relative;
      width: 15px;
      height: 2px;
      border-radius: 999px;
      background: currentColor;
    }

    .mobile-suite-menu-trigger span::before,
    .mobile-suite-menu-trigger span::after {
      content: '';
      position: absolute;
      left: 0;
      width: 15px;
      height: 2px;
      border-radius: 999px;
      background: currentColor;
    }

    .mobile-suite-menu-trigger span::before {
      top: -5px;
    }

    .mobile-suite-menu-trigger span::after {
      top: 5px;
    }

    .mobile-suite-menu-backdrop {
      position: fixed;
      inset: 0;
      z-index: 2147483001;
      width: auto;
      height: auto;
      border: 0;
      border-radius: 0;
      background: color-mix(in srgb, var(--bg, var(--og-bg)) 58%, transparent);
      backdrop-filter: blur(8px);
      pointer-events: auto;
    }

    .mobile-suite-menu-drawer {
      position: fixed;
      inset: 0;
      z-index: 2147483002;
      display: grid;
      align-content: start;
      gap: 10px;
      width: 100vw;
      height: 100vh;
      max-height: none;
      padding: max(10px, env(safe-area-inset-top)) 10px calc(10px + env(safe-area-inset-bottom));
      border: 0;
      background: color-mix(in srgb, var(--nav-bg, var(--og-nav-bg)) 96%, var(--surface, var(--og-surface)) 4%);
      box-shadow: var(--shadow, var(--og-shadow));
      overflow: auto;
      pointer-events: auto;
    }

    .mobile-suite-menu-drawer header,
    .mobile-suite-menu-drawer nav,
    .mobile-suite-menu-tools {
      display: flex;
      align-items: center;
      gap: 6px;
    }

    .mobile-suite-menu-drawer header {
      justify-content: space-between;
      min-height: 34px;
      color: var(--text, var(--og-text));
    }

    .mobile-suite-menu-drawer header button {
      width: 34px;
      min-width: 34px;
      height: 34px;
      min-height: 34px;
      padding: 0;
    }

    .mobile-suite-menu-drawer nav,
    .mobile-suite-menu-tools {
      flex-wrap: wrap;
      padding: 6px;
      border: 1px solid color-mix(in srgb, var(--border, var(--og-border)) 78%, transparent);
      border-radius: var(--field-radius, var(--og-field-radius));
      background: color-mix(in srgb, var(--surface-strong, var(--og-surface-strong)) 46%, transparent);
    }

    .mobile-suite-menu-drawer nav button,
    .mobile-suite-menu-tool,
    .mobile-suite-menu-tools :global(button) {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 7px;
      min-height: 30px;
      border: 0;
      border-radius: calc(var(--field-radius, var(--og-field-radius)) - 2px);
      background: transparent;
      color: var(--muted, var(--og-muted));
      padding: 0 10px;
      font-size: 0.8rem;
      font-weight: 900;
      letter-spacing: 0;
    }

    .mobile-suite-menu-drawer nav button.active,
    .mobile-suite-menu-drawer nav button:hover:not(:disabled),
    .mobile-suite-menu-drawer nav button:focus-visible,
    .mobile-suite-menu-tool:hover,
    .mobile-suite-menu-tool:focus-visible,
    .mobile-suite-menu-tools :global(button:hover:not(:disabled)),
    .mobile-suite-menu-tools :global(button:focus-visible) {
      background: color-mix(in srgb, var(--accent-soft, var(--og-accent-soft)) 64%, transparent);
      color: var(--text, var(--og-text));
      outline: none;
    }
  }
</style>
