import { mount } from 'svelte'
import StandaloneAudioApp from './StandaloneAudioApp.svelte'
import './audio.css'
import './styles.css'

mount(StandaloneAudioApp, {
  target: document.getElementById('app')!,
})
