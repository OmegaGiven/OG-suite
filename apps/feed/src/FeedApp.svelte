<script lang="ts">
  import type { CreateFeedFavoriteRequest, FeedActivityEvent, FeedFavorite } from '@og-suite/contracts'
  import type { RuntimeServices } from '@og-suite/runtime'
  import Icon from '@og-suite/ui/Icon'
  import MobileSuiteMenu from '@og-suite/ui/MobileSuiteMenu'
  import { onMount } from 'svelte'

  type SuiteNavItem = {
    id: string
    name: string
    disabled?: boolean
  }

  type SuiteOpenTarget = {
    appId: string
    targetKind: FeedActivityEvent['targetKind']
    targetId: string
    targetLabel: string
    requestId: number
  }

  export let services: RuntimeServices
  export let mode: 'suite' | 'standalone' = 'suite'
  export let suiteNavItems: SuiteNavItem[] = []
  export let activeSuiteAppId = ''
  export let onSuiteAppSelect: ((appId: string) => void) | undefined = undefined
  export let onOpenSuiteSettings: (() => void) | undefined = undefined
  export let onOpenActivityTarget: ((target: SuiteOpenTarget) => void) | undefined = undefined

  let activities: FeedActivityEvent[] = []
  let favorites: FeedFavorite[] = []
  let loading = true
  let error = ''

  $: favoriteKeys = new Set(favorites.map((favorite) => favoriteKey(favorite.targetKind, favorite.targetId)))

  onMount(() => {
    void refreshFeed()
    const interval = window.setInterval(() => void refreshFeed(false), 5000)
    return () => window.clearInterval(interval)
  })

  async function refreshFeed(showLoading = true) {
    if (showLoading) loading = true
    error = ''
    try {
      const [nextActivities, nextFavorites] = await Promise.all([
        services.api.get<FeedActivityEvent[]>('/api/v1/feed'),
        services.api.get<FeedFavorite[]>('/api/v1/feed/favorites'),
      ])
      activities = nextActivities
      favorites = nextFavorites
    } catch (feedError) {
      error = feedError instanceof Error ? feedError.message : 'Feed failed to load'
    } finally {
      loading = false
    }
  }

  async function addFavorite(request: CreateFeedFavoriteRequest) {
    await services.api.post<FeedFavorite>('/api/v1/feed/favorites', request)
    await refreshFeed(false)
  }

  async function removeFavorite(id: string) {
    await services.api.delete(`/api/v1/feed/favorites/${id}`)
    await refreshFeed(false)
  }

  function favoriteFromActivity(activity: FeedActivityEvent): CreateFeedFavoriteRequest {
    return {
      targetKind: activity.targetKind as CreateFeedFavoriteRequest['targetKind'],
      targetId: activity.targetId,
      label: activity.targetLabel,
      appId: activity.appId,
    }
  }

  function favoriteKey(kind: string, id: string) {
    return `${kind}:${id}`
  }

  function canFavorite(activity: FeedActivityEvent) {
    return ['note', 'folder', 'document', 'tool', 'recording'].includes(activity.targetKind)
  }

  function canOpen(activity: FeedActivityEvent) {
    return Boolean(activity.appId)
  }

  function openActivity(activity: FeedActivityEvent) {
    onOpenActivityTarget?.({
      appId: activity.appId,
      targetKind: activity.targetKind,
      targetId: activity.targetId,
      targetLabel: activity.targetLabel,
      requestId: Date.now(),
    })
  }

  function formatTime(value: string) {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(new Date(value))
  }

  function actionLabel(action: FeedActivityEvent['action']) {
    return action.replaceAll('_', ' ')
  }

  function selectSuiteApp(appId: string) {
    onSuiteAppSelect?.(appId)
  }
</script>

<article class="feed-app">
  <section class="feed-hero">
    <div>
      <p class="eyebrow">Suite history</p>
      <h1>Feed</h1>
    </div>
    <div class="feed-hero-actions">
      <button class="ghost-button" on:click={() => refreshFeed()} disabled={loading}>
        <Icon name="refresh" size={16} />
        <span>Refresh</span>
      </button>
      {#if mode === 'suite'}
        <MobileSuiteMenu
          title="Feed"
          navItems={suiteNavItems}
          activeAppId={activeSuiteAppId}
          onSelectApp={selectSuiteApp}
          onOpenSettings={onOpenSuiteSettings}
        >
          <button on:click={() => refreshFeed()} disabled={loading}>
            <Icon name="refresh" size={16} />
            <span>Refresh</span>
          </button>
        </MobileSuiteMenu>
      {/if}
    </div>
  </section>

  {#if error}
    <p class="feed-error">{error}</p>
  {/if}

  <section class="feed-section favorites-section" aria-labelledby="favorites-heading">
    <div class="section-heading">
      <h2 id="favorites-heading">Favorites</h2>
      <span>{favorites.length}</span>
    </div>

    <div class="favorite-grid">
      {#each favorites as favorite}
        <div class="favorite-item">
          <div>
            <p>{favorite.label}</p>
            <span>{favorite.targetKind} · {favorite.appId}</span>
          </div>
          <button class="icon-action" aria-label={`Remove ${favorite.label} from favorites`} on:click={() => removeFavorite(favorite.id)}>
            <Icon name="delete" size={15} />
          </button>
        </div>
      {:else}
        <p class="empty-copy">Favorite notes, folders, documents, or tools so they stay one tap away.</p>
      {/each}
    </div>
  </section>

  <section class="feed-section" aria-labelledby="timeline-heading">
    <div class="section-heading">
      <h2 id="timeline-heading">Activity</h2>
      <span>Newest first</span>
    </div>

    {#if loading}
      <p class="empty-copy">Loading activity...</p>
    {:else}
      <ol class="timeline">
        {#each activities as activity}
          <li class="timeline-item">
            <div class="timeline-marker"></div>
            <div class="timeline-body">
              <div class="timeline-topline">
                <strong>{activity.summary}</strong>
                <time datetime={activity.createdAt}>{formatTime(activity.createdAt)}</time>
              </div>
              <div class="timeline-meta">
                <span>{activity.actorName}</span>
                <span>{activity.appId}</span>
                <span>{actionLabel(activity.action)}</span>
                <span>{activity.targetKind}: {activity.targetLabel}</span>
              </div>
            </div>
            <div class="timeline-actions">
              {#if canOpen(activity)}
                <button
                  class="icon-action"
                  aria-label={`Open ${activity.targetLabel}`}
                  on:click={() => openActivity(activity)}
                >
                  <Icon name="open" size={15} />
                </button>
              {/if}
              {#if canFavorite(activity)}
                <button
                  class="icon-action"
                  aria-label={`Favorite ${activity.targetLabel}`}
                  disabled={favoriteKeys.has(favoriteKey(activity.targetKind, activity.targetId))}
                  on:click={() => addFavorite(favoriteFromActivity(activity))}
                >
                  <Icon name="star" size={15} />
                </button>
              {/if}
            </div>
          </li>
        {:else}
          <li class="empty-copy">No public activity yet. Edits and file actions will appear here after they reach the server.</li>
        {/each}
      </ol>
    {/if}
  </section>
</article>

<style>
  .feed-app {
    min-height: 100%;
    padding: var(--page-gutter, 16px);
    color: var(--text, var(--og-text));
  }

  .feed-hero,
  .feed-section {
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-panel-radius));
    background: var(--panel-surface, var(--og-surface));
    box-shadow: var(--shadow, var(--og-shadow));
    backdrop-filter: blur(18px);
  }

  .feed-hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 12px);
    padding: 14px;
  }

  .feed-hero-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  @media (max-width: 760px) {
    .feed-app {
      padding: 0;
    }

    .feed-hero,
    .feed-section {
      border-left: 0;
      border-right: 0;
      border-radius: 0;
    }

    .feed-hero-actions > .ghost-button {
      display: none;
    }
  }

  .eyebrow,
  h1,
  h2,
  p {
    margin: 0;
  }

  .eyebrow {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
  }

  h1 {
    font-size: 22px;
    font-weight: 720;
  }

  h2 {
    font-size: 15px;
    font-weight: 700;
  }

  .feed-section {
    margin-top: var(--space-md, 12px);
    padding: 12px;
  }

  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }

  .section-heading span,
  .timeline-meta,
  .favorite-item span,
  .empty-copy {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
  }

  .favorite-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 8px;
  }

  .favorite-item,
  .timeline-item {
    border: 1px solid color-mix(in srgb, var(--border, var(--og-border)) 74%, transparent);
    border-radius: max(6px, calc(var(--panel-radius, var(--og-panel-radius)) - 8px));
    background: var(--panel-surface-subtle, var(--og-surface-subtle));
  }

  .favorite-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 54px;
    padding: 9px;
  }

  .favorite-item p {
    font-weight: 650;
  }

  .ghost-button,
  .icon-action {
    border: 1px solid color-mix(in srgb, var(--border, var(--og-border)) 72%, transparent);
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    cursor: pointer;
  }

  .ghost-button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 6px 9px;
  }

  .icon-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    width: 30px;
    height: 30px;
  }

  .ghost-button:hover:not(:disabled),
  .icon-action:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent, var(--og-accent)) 42%, var(--border, var(--og-border)));
  }

  button:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .timeline {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .timeline-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    padding: 9px;
  }

  .timeline-marker {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--accent, var(--og-accent));
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent-soft, var(--og-accent-soft)) 70%, transparent);
  }

  .timeline-body {
    min-width: 0;
  }

  .timeline-actions {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 6px;
    min-width: max-content;
  }

  .timeline-topline {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    min-width: 0;
  }

  .timeline-topline strong {
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .timeline-topline time {
    flex: 0 0 auto;
    color: var(--muted, var(--og-muted));
    font-size: 11px;
  }

  .timeline-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 8px;
    margin-top: 4px;
  }

  .feed-error {
    margin-top: 10px;
    color: var(--danger, #dc2626);
    font-size: 13px;
  }

  @media (max-width: 760px) {
    .feed-app {
      padding: 0;
    }

    .feed-hero,
    .feed-section {
      border-left: 0;
      border-right: 0;
      border-radius: 0;
    }

    .timeline-item {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .timeline-actions {
      align-self: start;
    }

    .timeline-marker {
      display: none;
    }
  }
</style>
