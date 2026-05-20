<script lang="ts">
  import type {
    AudioRecording,
    AudioFolder,
    AudioTranscript,
    CreateAudioFolderRequest,
    CreateAudioRecordingRequest,
    UploadAudioRequest,
    UpdateAudioRecordingRequest,
  } from '@og-suite/contracts'
  import type { RuntimeServices } from '@og-suite/runtime'
  import ActionBar from '@og-suite/ui/ActionBar'
  import ActionButton from '@og-suite/ui/ActionButton'
  import FileNavigator from '@og-suite/ui/FileNavigator'
  import type { FileNavigatorFolder, FileNavigatorItem } from '@og-suite/ui/FileNavigator'
  import Icon from '@og-suite/ui/Icon'
  import MobileSuiteMenu from '@og-suite/ui/MobileSuiteMenu'
  import { onMount } from 'svelte'

  export let services: RuntimeServices
  export let mode: 'suite' | 'standalone' = 'suite'
  export let suiteNavItems: SuiteNavItem[] = []
  export let activeSuiteAppId = ''
  export let onSuiteAppSelect: ((appId: string) => void) | undefined = undefined
  export let onOpenSuiteSettings: (() => void) | undefined = undefined
  export let openTarget: SuiteOpenTarget | null = null

  type SuiteNavItem = {
    id: string
    name: string
    disabled?: boolean
  }

  type SuiteOpenTarget = {
    appId: string
    targetKind: string
    targetId: string
    targetLabel: string
    requestId: number
  }

  type LocalAudioDraft = {
    localId: string
    title: string
    mimeType: string
    durationMs: number
    sizeBytes: number
    createdAt: string
    backedUpRecordingId?: string
    syncedAt?: string
  }

  const draftsKey = 'og-suite:audio:drafts'
  const clearBackedUpKey = 'og-suite:audio:clear-backed-up'
  const dbName = 'og-suite-audio'
  const storeName = 'draft-blobs'

  let recordings: AudioRecording[] = []
  let folders: AudioFolder[] = []
  let localDrafts: LocalAudioDraft[] = []
  let transcript: AudioTranscript | null = null
  let selectedRecordingId = ''
  let mediaRecorder: MediaRecorder | null = null
  let stream: MediaStream | null = null
  let chunks: Blob[] = []
  let recordingState: 'idle' | 'recording' | 'paused' | 'syncing' = 'idle'
  let recordingStartedAt = 0
  let elapsedMs = 0
  let elapsedTimer: number | undefined
  let audioContext: AudioContext | null = null
  let analyser: AnalyserNode | null = null
  let audioMeterFrame = 0
  let audioLevel = 0
  let statusMessage = ''
  let error = ''
  let clearBackedUpLocalAudio = true
  let editingTitle = false
  let titleDraft = ''
  let activeFolderPath = '/'
  let selectedFolderPath = ''
  let collapsedFolderPaths: string[] = []
  let searchOpen = false
  let searchQuery = ''
  let uploadInputElement: HTMLInputElement | null = null
  let handledOpenTargetKey = ''

  $: selectedRecording = recordings.find((recording) => recording.id === selectedRecordingId) ?? recordings[0]
  $: recorderOnly = mode === 'standalone'
  $: selectedAudioUrl = selectedRecording ? backendUrl(`/api/v1/audio/recordings/${selectedRecording.id}/audio`) : ''
  $: recordingControlLabel = recordingState === 'idle'
    ? 'Start recording'
    : recordingState === 'recording'
      ? 'Pause recording'
      : recordingState === 'paused'
        ? 'Resume recording'
        : 'Recording is syncing'
  $: filteredRecordings = recordings.filter((recording) => {
    const query = searchQuery.trim().toLowerCase()
    if (!query) return true
    return [recording.title, recording.path, recording.status, recording.mimeType]
      .some((value) => value.toLowerCase().includes(query))
  })
  $: navigatorItems = filteredRecordings.map((recording): FileNavigatorItem => ({
    id: recording.id,
    title: recording.title,
    path: recording.path,
    meta: `${formatDuration(recording.durationMs)} · ${formatBytes(recording.sizeBytes)} · ${recording.status}`,
  }))
  $: navigatorFolders = folders.map((folder): FileNavigatorFolder => ({
    id: folder.id,
    path: folder.path,
    name: folder.name,
  }))
  $: selectedAudioFolder = selectedFolderPath
    ? folders.find((folder) => normalizeFolderPath(folder.path) === normalizeFolderPath(selectedFolderPath))
    : undefined
  $: selectedFolderRecordings = selectedFolderPath
    ? recordings.filter((recording) => isSameOrNestedPath(recording.path, selectedFolderPath))
    : []
  $: selectedFolderTree = selectedFolderPath
    ? folders.filter((folder) => isSameOrNestedPath(folder.path, selectedFolderPath))
    : []
  $: selectedFolderCanDelete = Boolean(selectedAudioFolder && selectedFolderRecordings.length === 0)
  $: if (openTarget?.appId === 'audio') {
    void openSuiteTarget(openTarget, recordings.length)
  }

  onMount(() => {
    clearBackedUpLocalAudio = localStorage.getItem(clearBackedUpKey) !== 'false'
    loadLocalDrafts()
    void refreshRecordings()
    void syncLocalDrafts()
    return () => stopStream()
  })

  async function refreshRecordings() {
    try {
      recordings = await services.api.get<AudioRecording[]>('/api/v1/audio/recordings')
      folders = await services.api.get<AudioFolder[]>('/api/v1/audio/folders')
      if (!selectedRecordingId && recordings[0]) selectedRecordingId = recordings[0].id
      if (selectedRecording) await loadTranscript(selectedRecording.id)
    } catch (refreshError) {
      error = refreshError instanceof Error ? refreshError.message : 'Audio recordings failed to load'
    }
  }

  async function loadTranscript(recordingId: string) {
    try {
      transcript = await services.api.get<AudioTranscript>(`/api/v1/audio/recordings/${recordingId}/transcript`)
    } catch {
      transcript = null
    }
  }

  function beginRenameSelectedRecording() {
    if (!selectedRecording) return
    titleDraft = selectedRecording.title
    editingTitle = true
  }

  async function saveSelectedRecordingTitle() {
    if (!selectedRecording) return
    const title = titleDraft.trim()
    if (!title) return
    error = ''
    try {
      const updated = await services.api.patch<AudioRecording>(`/api/v1/audio/recordings/${selectedRecording.id}`, {
        title,
      } satisfies UpdateAudioRecordingRequest)
      recordings = recordings.map((recording) => recording.id === updated.id ? updated : recording)
      editingTitle = false
      statusMessage = 'Recording renamed.'
    } catch (renameError) {
      error = renameError instanceof Error ? renameError.message : 'Rename failed'
    }
  }

  async function createFolder() {
    const name = window.prompt('Folder name')
    if (!name?.trim()) return
    const path = normalizeFolderPath(`${activeFolderPath}/${name.trim()}`)
    const folder = await services.api.post<AudioFolder>('/api/v1/audio/folders', {
      path,
    } satisfies CreateAudioFolderRequest)
    folders = [...folders.filter((item) => item.id !== folder.id), folder]
    activeFolderPath = folder.path
    selectedFolderPath = folder.path
  }

  async function selectFolder(path: string) {
    activeFolderPath = normalizeFolderPath(path)
    selectedFolderPath = activeFolderPath === '/' ? '' : activeFolderPath
  }

  async function moveRecording(recordingId: string, path: string) {
    const recording = recordings.find((item) => item.id === recordingId)
    if (!recording) return
    const updated = await services.api.patch<AudioRecording>(`/api/v1/audio/recordings/${recordingId}`, {
      path: normalizeFolderPath(path),
    } satisfies UpdateAudioRecordingRequest)
    recordings = recordings.map((item) => item.id === updated.id ? updated : item)
    activeFolderPath = updated.path
  }

  async function moveFolder(sourcePath: string, targetPath: string) {
    const source = normalizeFolderPath(sourcePath)
    const target = normalizeFolderPath(targetPath)
    if (source === '/' || source === target || target.startsWith(`${source}/`)) return
    const nextPath = normalizeFolderPath(`${target}/${folderName(source)}`)
    if (source === nextPath) return
    const affectedFolders = folders.filter((folder) => {
      const current = normalizeFolderPath(folder.path)
      return current === source || current.startsWith(`${source}/`)
    })
    const affectedRecordings = recordings.filter((recording) => {
      const current = normalizeFolderPath(recording.path)
      return current === source || current.startsWith(`${source}/`)
    })

    for (const folder of affectedFolders) {
      const nextFolderPath = remapPath(normalizeFolderPath(folder.path), source, nextPath)
      const updatedFolder = await services.api.post<AudioFolder>('/api/v1/audio/folders', { path: nextFolderPath })
      folders = [...folders.filter((item) => item.id !== updatedFolder.id), updatedFolder]
    }
    for (const recording of affectedRecordings) {
      await moveRecording(recording.id, remapPath(normalizeFolderPath(recording.path), source, nextPath))
    }
    for (const folder of affectedFolders) {
      await services.api.delete(`/api/v1/audio/folders/${folder.id}`)
    }
    activeFolderPath = nextPath
    selectedFolderPath = nextPath
    await refreshRecordings()
  }

  function selectRecording(id: string) {
    selectedRecordingId = id
    selectedFolderPath = ''
    void loadTranscript(id)
  }

  async function openSuiteTarget(target: SuiteOpenTarget, _recordingCount: number) {
    const key = `${target.requestId}:${target.appId}:${target.targetKind}:${target.targetId}`
    if (handledOpenTargetKey === key) return
    handledOpenTargetKey = key

    if (target.targetKind !== 'recording') return
    let recording = recordings.find((item) => item.id === target.targetId)
    if (!recording) {
      await refreshRecordings()
      recording = recordings.find((item) => item.id === target.targetId)
    }
    if (recording) {
      selectedRecordingId = recording.id
      selectedFolderPath = ''
      activeFolderPath = normalizeFolderPath(recording.path)
      await loadTranscript(recording.id)
      statusMessage = `Opened ${recording.title}.`
    } else {
      statusMessage = `Could not find ${target.targetLabel}.`
    }
  }

  function toggleFolder(path: string) {
    const normalized = normalizeFolderPath(path)
    collapsedFolderPaths = collapsedFolderPaths.includes(normalized)
      ? collapsedFolderPaths.filter((item) => item !== normalized)
      : [...collapsedFolderPaths, normalized]
  }

  function selectSuiteApp(appId: string) {
    onSuiteAppSelect?.(appId)
  }

  function triggerUpload() {
    uploadInputElement?.click()
  }

  async function retranscribeSelectedRecording() {
    if (!selectedRecording) return
    error = ''
    statusMessage = 'Queued for retranscription...'
    try {
      await services.api.post(`/api/v1/audio/recordings/${selectedRecording.id}/transcript`, {})
      await refreshRecordings()
      await loadTranscript(selectedRecording.id)
      statusMessage = 'Retranscription queued.'
    } catch (retranscribeError) {
      error = retranscribeError instanceof Error ? retranscribeError.message : 'Retranscription failed'
    }
  }

  async function deleteSelectedRecording() {
    if (!selectedRecording) return
    if (!window.confirm(`Delete "${selectedRecording.title}"?`)) return
    error = ''
    const deletedId = selectedRecording.id
    try {
      await services.api.delete(`/api/v1/audio/recordings/${deletedId}`)
      recordings = recordings.filter((recording) => recording.id !== deletedId)
      selectedRecordingId = recordings[0]?.id ?? ''
      transcript = null
      if (selectedRecordingId) await loadTranscript(selectedRecordingId)
      statusMessage = 'Recording deleted.'
    } catch (deleteError) {
      error = deleteError instanceof Error ? deleteError.message : 'Delete failed'
    }
  }

  async function deleteSelectedAudioItem() {
    if (selectedFolderPath) {
      await deleteSelectedFolder()
      return
    }
    await deleteSelectedRecording()
  }

  async function deleteSelectedFolder() {
    if (!selectedAudioFolder) return
    if (selectedFolderRecordings.length > 0) {
      statusMessage = 'Move or delete recordings inside this folder first.'
      return
    }
    const folderCount = selectedFolderTree.length
    const label = selectedFolderPath
    if (!window.confirm(`Delete "${label}"${folderCount > 1 ? ` and ${folderCount - 1} nested empty folder${folderCount === 2 ? '' : 's'}` : ''}?`)) return
    error = ''
    try {
      for (const folder of selectedFolderTree.sort((left, right) => right.path.length - left.path.length)) {
        await services.api.delete(`/api/v1/audio/folders/${folder.id}`)
      }
      folders = folders.filter((folder) => !selectedFolderTree.some((deleted) => deleted.id === folder.id))
      selectedFolderPath = ''
      activeFolderPath = '/'
      statusMessage = 'Folder deleted.'
      await refreshRecordings()
    } catch (deleteError) {
      error = deleteError instanceof Error ? deleteError.message : 'Delete failed'
    }
  }

  function downloadTranscript(format: 'vtt' | 'srt') {
    if (!selectedRecording) return
    window.location.href = backendUrl(`/api/v1/audio/recordings/${selectedRecording.id}/transcript.${format}`)
  }

  function downloadSelectedRecording() {
    if (!selectedRecording?.assetRef) return
    const safeTitle = (selectedRecording.title.trim() || 'Recording').replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '') || 'recording'
    const link = document.createElement('a')
    link.href = selectedAudioUrl
    link.download = `${safeTitle}.${mediaExtension(selectedRecording.mimeType)}`
    link.click()
  }

  function mediaExtension(mimeType: string) {
    if (mimeType.includes('mpeg')) return 'mp3'
    if (mimeType.includes('mp4')) return 'mp4'
    if (mimeType.includes('ogg')) return 'ogg'
    if (mimeType.includes('wav')) return 'wav'
    if (mimeType.includes('webm')) return 'webm'
    return 'audio'
  }

  function activateRecordingIndicator() {
    if (recordingState === 'idle') {
      void startRecording()
    } else if (recordingState === 'recording') {
      pauseRecording()
    } else if (recordingState === 'paused') {
      resumeRecording()
    }
  }

  async function startRecording() {
    error = ''
    statusMessage = ''
    chunks = []
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      startAudioMeter(stream)
      mediaRecorder = new MediaRecorder(stream)
      mediaRecorder.addEventListener('dataavailable', (event) => {
        if (event.data.size > 0) chunks = [...chunks, event.data]
      })
      mediaRecorder.addEventListener('stop', () => {
        void persistAndSyncRecording()
      })
      recordingStartedAt = Date.now()
      elapsedMs = 0
      elapsedTimer = window.setInterval(() => {
        elapsedMs = Date.now() - recordingStartedAt
      }, 250)
      mediaRecorder.start()
      recordingState = 'recording'
    } catch (recordError) {
      error = recordError instanceof Error ? recordError.message : 'Microphone permission was not granted'
      stopStream()
    }
  }

  function pauseRecording() {
    if (mediaRecorder?.state === 'recording') {
      mediaRecorder.pause()
      recordingState = 'paused'
    }
  }

  function resumeRecording() {
    if (mediaRecorder?.state === 'paused') {
      mediaRecorder.resume()
      recordingState = 'recording'
    }
  }

  function stopRecording() {
    if (!mediaRecorder || mediaRecorder.state === 'inactive') return
    recordingState = 'syncing'
    mediaRecorder.stop()
    if (elapsedTimer) window.clearInterval(elapsedTimer)
    elapsedMs = Date.now() - recordingStartedAt
    stopStream()
  }

  function stopStream() {
    stopAudioMeter()
    stream?.getTracks().forEach((track) => track.stop())
    stream = null
  }

  function startAudioMeter(inputStream: MediaStream) {
    stopAudioMeter()
    audioContext = new AudioContext()
    analyser = audioContext.createAnalyser()
    analyser.fftSize = 256
    const source = audioContext.createMediaStreamSource(inputStream)
    source.connect(analyser)
    const data = new Uint8Array(analyser.fftSize)

    const updateMeter = () => {
      if (!analyser) return
      analyser.getByteTimeDomainData(data)
      let sum = 0
      for (const value of data) {
        const centered = (value - 128) / 128
        sum += centered * centered
      }
      const rms = Math.sqrt(sum / data.length)
      audioLevel = Math.min(1, Math.max(0, rms * 4))
      audioMeterFrame = requestAnimationFrame(updateMeter)
    }

    void audioContext.resume()
    updateMeter()
  }

  function stopAudioMeter() {
    if (audioMeterFrame) cancelAnimationFrame(audioMeterFrame)
    audioMeterFrame = 0
    analyser = null
    void audioContext?.close()
    audioContext = null
    audioLevel = 0
  }

  async function persistAndSyncRecording() {
    const mimeType = mediaRecorder?.mimeType || 'audio/webm'
    const blob = new Blob(chunks, { type: mimeType })
    const localId = crypto.randomUUID()
    const draft: LocalAudioDraft = {
      localId,
      title: `Recording ${new Date().toLocaleString()}`,
      mimeType,
      durationMs: elapsedMs,
      sizeBytes: blob.size,
      createdAt: new Date().toISOString(),
    }
    await putBlob(localId, blob)
    localDrafts = [draft, ...localDrafts]
    saveLocalDrafts()
    statusMessage = 'Saved locally. Syncing to backend...'
    await syncDraft(draft)
    recordingState = 'idle'
    chunks = []
    mediaRecorder = null
  }

  async function syncLocalDrafts() {
    for (const draft of localDrafts.filter((item) => !item.backedUpRecordingId)) {
      await syncDraft(draft)
    }
  }

  async function uploadMediaFiles(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    const files = Array.from(input.files ?? [])
    input.value = ''
    for (const file of files) {
      const localId = crypto.randomUUID()
      const draft: LocalAudioDraft = {
        localId,
        title: file.name.replace(/\.[^.]+$/, '') || `Upload ${new Date().toLocaleString()}`,
        mimeType: file.type || 'application/octet-stream',
        durationMs: await readMediaDuration(file),
        sizeBytes: file.size,
        createdAt: new Date().toISOString(),
      }
      await putBlob(localId, file)
      localDrafts = [draft, ...localDrafts]
      saveLocalDrafts()
      statusMessage = `Saved "${draft.title}" locally. Syncing to backend...`
      await syncDraft(draft)
    }
  }

  async function syncDraft(draft: LocalAudioDraft) {
    const blob = await getBlob(draft.localId)
    if (!blob) return
    try {
      const request: CreateAudioRecordingRequest = {
        title: draft.title,
        path: activeFolderPath,
        mimeType: draft.mimeType,
        durationMs: draft.durationMs,
        sizeBytes: draft.sizeBytes,
      }
      const recording = await services.api.post<AudioRecording>('/api/v1/audio/recordings', request)
      const dataUrl = await blobToDataUrl(blob)
      await services.api.post<AudioRecording>(`/api/v1/audio/recordings/${recording.id}/audio`, {
        dataUrl,
        mimeType: draft.mimeType,
        sizeBytes: draft.sizeBytes,
      } satisfies UploadAudioRequest)
      localDrafts = localDrafts.map((item) =>
        item.localId === draft.localId
          ? { ...item, backedUpRecordingId: recording.id, syncedAt: new Date().toISOString() }
          : item,
      )
      if (clearBackedUpLocalAudio) {
        await deleteBlob(draft.localId)
        localDrafts = localDrafts.filter((item) => item.localId !== draft.localId)
      }
      saveLocalDrafts()
      statusMessage = clearBackedUpLocalAudio ? 'Backed up. Local audio copy removed.' : 'Backed up. Local audio copy retained.'
      await refreshRecordings()
    } catch (syncError) {
      statusMessage = 'Saved locally. Sync will retry when the backend is available.'
      error = syncError instanceof Error ? syncError.message : ''
    }
  }

  function updateClearBackedUpLocalAudio(value: boolean) {
    clearBackedUpLocalAudio = value
    localStorage.setItem(clearBackedUpKey, String(value))
  }

  function loadLocalDrafts() {
    const raw = localStorage.getItem(draftsKey)
    localDrafts = raw ? JSON.parse(raw) as LocalAudioDraft[] : []
  }

  function saveLocalDrafts() {
    localStorage.setItem(draftsKey, JSON.stringify(localDrafts))
  }

  function formatDuration(ms: number) {
    const totalSeconds = Math.max(0, Math.round(ms / 1000))
    const minutes = Math.floor(totalSeconds / 60).toString().padStart(2, '0')
    const seconds = (totalSeconds % 60).toString().padStart(2, '0')
    return `${minutes}:${seconds}`
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  function normalizeFolderPath(path: string) {
    const trimmed = path.trim()
    if (!trimmed || trimmed === '/') return '/'
    return `/${trimmed.replace(/^\/+|\/+$/g, '')}`
  }

  function folderName(path: string) {
    const normalized = normalizeFolderPath(path)
    return normalized === '/' ? 'Root' : normalized.split('/').filter(Boolean).at(-1) ?? normalized
  }

  function remapPath(path: string, source: string, target: string) {
    const normalized = normalizeFolderPath(path)
    const normalizedSource = normalizeFolderPath(source)
    const normalizedTarget = normalizeFolderPath(target)
    if (normalized === normalizedSource) return normalizedTarget
    return normalizeFolderPath(`${normalizedTarget}/${normalized.slice(normalizedSource.length)}`)
  }

  function isSameOrNestedPath(path: string, parent: string) {
    const normalized = normalizeFolderPath(path)
    const normalizedParent = normalizeFolderPath(parent)
    return normalized === normalizedParent || normalized.startsWith(`${normalizedParent}/`)
  }

  function readMediaDuration(file: File) {
    return new Promise<number>((resolve) => {
      const element = file.type.startsWith('video/') ? document.createElement('video') : document.createElement('audio')
      const url = URL.createObjectURL(file)
      element.preload = 'metadata'
      element.onloadedmetadata = () => {
        const duration = Number.isFinite(element.duration) ? Math.round(element.duration * 1000) : 0
        URL.revokeObjectURL(url)
        resolve(duration)
      }
      element.onerror = () => {
        URL.revokeObjectURL(url)
        resolve(0)
      }
      element.src = url
    })
  }

  function backendUrl(path: string) {
    const apiHost = typeof window === 'undefined' || window.location.hostname === 'localhost' ? '127.0.0.1' : window.location.hostname
    const defaultApiUrl =
      typeof window === 'undefined' ? 'http://127.0.0.1:8080' : `http://${apiHost}:8080`
    const baseUrl = import.meta.env.VITE_OG_API_URL ?? defaultApiUrl
    return `${baseUrl}${path}`
  }

  function blobToDataUrl(blob: Blob) {
    return new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.addEventListener('load', () => resolve(String(reader.result)))
      reader.addEventListener('error', () => reject(reader.error))
      reader.readAsDataURL(blob)
    })
  }

  function openDb() {
    return new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open(dbName, 1)
      request.onupgradeneeded = () => {
        request.result.createObjectStore(storeName)
      }
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  async function putBlob(id: string, blob: Blob) {
    const db = await openDb()
    await idbRequest(db.transaction(storeName, 'readwrite').objectStore(storeName).put(blob, id))
    db.close()
  }

  async function getBlob(id: string) {
    const db = await openDb()
    const blob = await idbRequest<Blob | undefined>(db.transaction(storeName, 'readonly').objectStore(storeName).get(id))
    db.close()
    return blob
  }

  async function deleteBlob(id: string) {
    const db = await openDb()
    await idbRequest(db.transaction(storeName, 'readwrite').objectStore(storeName).delete(id))
    db.close()
  }

  function idbRequest<T = unknown>(request: IDBRequest<T>) {
    return new Promise<T>((resolve, reject) => {
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }
</script>

<section class:recorder-only={recorderOnly} class="audio-app">
  <div class="recorder-panel">
    <div class="recorder-heading">
      <div class="recorder-heading-actions">
        <div class:active={recordingState === 'recording'} class="recording-light"></div>
        {#if mode === 'suite'}
          <MobileSuiteMenu
            title="Audio"
            navItems={suiteNavItems}
            activeAppId={activeSuiteAppId}
            onSelectApp={selectSuiteApp}
            onOpenSettings={onOpenSuiteSettings}
          >
            {#if !recorderOnly}
              <div class="mobile-library-menu">
                <ActionBar ariaLabel="Recording file actions" className="panel-actions library-action-bar">
                  <ActionButton icon="search" label="Search" iconOnly on:click={() => searchOpen = !searchOpen} />
                  <ActionButton icon="new-folder" label="New folder" iconOnly on:click={createFolder} />
                  <ActionButton icon="upload" label="Upload" iconOnly on:click={triggerUpload} />
                  <ActionButton icon="rename" label="Rename selected recording" iconOnly disabled={Boolean(selectedFolderPath) || !selectedRecording} on:click={beginRenameSelectedRecording} />
                  <ActionButton icon="download" label="Download selected recording" iconOnly disabled={Boolean(selectedFolderPath) || !selectedRecording?.assetRef} on:click={downloadSelectedRecording} />
                  <ActionButton
                    icon="delete"
                    label={selectedFolderPath ? 'Delete selected empty folder' : 'Delete selected recording'}
                    iconOnly
                    tone="danger"
                    disabled={selectedFolderPath ? !selectedFolderCanDelete : !selectedRecording}
                    on:click={deleteSelectedAudioItem}
                  />
                  <ActionButton icon="refresh" label="Refresh" iconOnly on:click={refreshRecordings} />
                </ActionBar>
                {#if searchOpen}
                  <label class="library-search">
                    <Icon name="search" size={16} />
                    <input bind:value={searchQuery} type="search" placeholder="Search recordings" aria-label="Search recordings" />
                  </label>
                {/if}
                <FileNavigator
                  folders={navigatorFolders}
                  items={navigatorItems}
                  selectedItemId={selectedRecordingId}
                  {activeFolderPath}
                  {selectedFolderPath}
                  {collapsedFolderPaths}
                  itemLabel="recording"
                  onSelectItem={selectRecording}
                  onSelectFolder={selectFolder}
                  onMoveItem={moveRecording}
                  onMoveFolder={moveFolder}
                  onToggleFolder={toggleFolder}
                />
              </div>
            {/if}
            <button on:click={syncLocalDrafts} disabled={!localDrafts.some((draft) => !draft.backedUpRecordingId)}>
              <Icon name="sync" size={16} />
              <span>Sync local</span>
            </button>
            <button on:click={refreshRecordings}>
              <Icon name="refresh" size={16} />
              <span>Refresh</span>
            </button>
          </MobileSuiteMenu>
        {/if}
      </div>
    </div>

    <div class="timer">{formatDuration(elapsedMs)}</div>

    <button
      type="button"
      class:active={recordingState === 'recording' || recordingState === 'paused'}
      class:paused={recordingState === 'paused'}
      class="input-meter"
      style={`--input-level: ${audioLevel}`}
      aria-label={recordingControlLabel}
      aria-pressed={recordingState === 'recording'}
      title={recordingControlLabel}
      disabled={recordingState === 'syncing'}
      on:click={activateRecordingIndicator}
    >
      <span></span>
      <Icon name="microphone" size={20} />
      <span></span>
    </button>

    {#if recordingState !== 'idle'}
      <ActionBar ariaLabel="Recording controls" align="center" className="recording-controls">
      {#if recordingState === 'recording'}
        <ActionButton label="Pause" on:click={pauseRecording} />
        <ActionButton label="Stop" tone="danger" on:click={stopRecording} />
      {:else if recordingState === 'paused'}
        <ActionButton label="Resume" tone="primary" on:click={resumeRecording} />
        <ActionButton label="Stop" tone="danger" on:click={stopRecording} />
      {:else}
        <ActionButton label="Syncing" disabled />
      {/if}
      </ActionBar>
    {/if}

    <label class="behavior-row">
      <input
        type="checkbox"
        checked={clearBackedUpLocalAudio}
        on:change={(event) => updateClearBackedUpLocalAudio(event.currentTarget.checked)}
      />
      <span>Remove local audio after backend backup</span>
    </label>

    <div class="sync-row">
      <ActionButton label="Sync local recordings" icon="sync" on:click={syncLocalDrafts} disabled={!localDrafts.some((draft) => !draft.backedUpRecordingId)} />
      <span>{localDrafts.length} local</span>
    </div>

    <label class="upload-action">
      <input bind:this={uploadInputElement} type="file" accept="audio/*,video/*" multiple on:change={uploadMediaFiles} />
      <span>Upload audio/video</span>
    </label>

    {#if statusMessage}
      <p class="status-copy">{statusMessage}</p>
    {/if}
    {#if error}
      <p class="error-copy">{error}</p>
    {/if}
  </div>

  {#if !recorderOnly}
    <div class="library-panel">
      <ActionBar ariaLabel="Recording file actions" className="panel-actions library-action-bar">
        <ActionButton icon="search" label="Search" iconOnly on:click={() => searchOpen = !searchOpen} />
        <ActionButton icon="new-folder" label="New folder" iconOnly on:click={createFolder} />
        <ActionButton icon="upload" label="Upload" iconOnly on:click={triggerUpload} />
        <ActionButton icon="rename" label="Rename selected recording" iconOnly disabled={Boolean(selectedFolderPath) || !selectedRecording} on:click={beginRenameSelectedRecording} />
        <ActionButton icon="download" label="Download selected recording" iconOnly disabled={Boolean(selectedFolderPath) || !selectedRecording?.assetRef} on:click={downloadSelectedRecording} />
        <ActionButton
          icon="delete"
          label={selectedFolderPath ? 'Delete selected empty folder' : 'Delete selected recording'}
          iconOnly
          tone="danger"
          disabled={selectedFolderPath ? !selectedFolderCanDelete : !selectedRecording}
          on:click={deleteSelectedAudioItem}
        />
        <ActionButton icon="refresh" label="Refresh" iconOnly on:click={refreshRecordings} />
      </ActionBar>
      {#if searchOpen}
        <label class="library-search">
          <Icon name="search" size={16} />
          <input bind:value={searchQuery} type="search" placeholder="Search recordings" aria-label="Search recordings" />
        </label>
      {/if}
      <FileNavigator
        folders={navigatorFolders}
        items={navigatorItems}
        selectedItemId={selectedRecordingId}
        {activeFolderPath}
        {selectedFolderPath}
        {collapsedFolderPaths}
        itemLabel="recording"
        onSelectItem={selectRecording}
        onSelectFolder={selectFolder}
        onMoveItem={moveRecording}
        onMoveFolder={moveFolder}
        onToggleFolder={toggleFolder}
      />
    </div>

    <div class="transcript-panel">
      <div class="panel-title transcript-title">
        <div>
          {#if editingTitle}
            <form class="rename-form" on:submit|preventDefault={saveSelectedRecordingTitle}>
              <input bind:value={titleDraft} aria-label="Recording title" />
              <ActionButton label="Save" icon="save" type="submit" />
              <ActionButton label="Cancel" type="button" on:click={() => editingTitle = false} />
            </form>
          {:else}
            <h2>{selectedRecording?.title ?? 'Transcript'}</h2>
          {/if}
          {#if transcript}<span>{transcript.status}</span>{/if}
        </div>
        <ActionBar ariaLabel="Transcript actions" className="panel-actions">
          <ActionButton icon="refresh" label="Retranscribe" disabled={!selectedRecording?.assetRef} on:click={retranscribeSelectedRecording} />
          <ActionButton icon="download" label="VTT" disabled={!transcript?.segments.length} on:click={() => downloadTranscript('vtt')} />
          <ActionButton icon="download" label="SRT" disabled={!transcript?.segments.length} on:click={() => downloadTranscript('srt')} />
        </ActionBar>
      </div>
      {#if selectedRecording?.assetRef}
        <audio class="audio-player" controls src={selectedAudioUrl}>
          <track kind="captions" />
        </audio>
      {/if}
      {#if transcript?.segments.length}
        <div class="transcript-list">
          {#each transcript.segments as segment}
            <article class="transcript-segment">
              <div>
                <strong>{segment.speakerLabel ?? `Channel ${segment.channel ?? 1}`}</strong>
                <span>{formatDuration(segment.startMs)}-{formatDuration(segment.endMs)}</span>
              </div>
              <p>{segment.text}</p>
            </article>
          {/each}
        </div>
      {:else}
        <p class="empty-copy">Transcript will appear here after the backend provider processes the audio.</p>
      {/if}
    </div>
  {/if}
</section>
