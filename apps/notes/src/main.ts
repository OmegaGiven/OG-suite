import { mount } from 'svelte'
import NotesApp from './NotesApp.svelte'
import { createStandaloneRuntime } from './runtime'
import './styles.css'

mount(NotesApp, {
  target: document.getElementById('app')!,
  props: {
    services: createStandaloneRuntime(),
    mode: 'standalone',
  },
})

