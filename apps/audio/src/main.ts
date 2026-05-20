import { mount } from 'svelte'
import AudioApp from './AudioApp.svelte'
import { createStandaloneRuntime } from './runtime'
import './audio.css'
import './styles.css'

mount(AudioApp, {
  target: document.getElementById('app')!,
  props: {
    services: createStandaloneRuntime(),
    mode: 'standalone',
  },
})
