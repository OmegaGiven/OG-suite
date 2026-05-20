import { mount } from 'svelte'
import SuiteApp from './SuiteApp.svelte'
import { createSuiteRuntime } from './runtime'
import '../../audio/src/audio.css'
import '../../files/src/files.css'
import '../../notes/src/styles.css'
import './suite.css'

mount(SuiteApp, {
  target: document.getElementById('app')!,
  props: {
    services: createSuiteRuntime(),
  },
})
