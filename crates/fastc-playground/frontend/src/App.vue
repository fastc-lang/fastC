<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import Editor from './components/Editor.vue'
import OutputPanel from './components/OutputPanel.vue'
import Terminal from './components/Terminal.vue'
import Toolbar from './components/Toolbar.vue'
import { usePlaygroundStore } from './stores/playground'

const store = usePlaygroundStore()

// Terminal resizing
const isResizing = ref(false)
const startY = ref(0)
const startHeight = ref(0)

const terminalStyle = computed(() => ({
  height: `${store.terminalHeight}px`
}))

function startResize(e: MouseEvent) {
  isResizing.value = true
  startY.value = e.clientY
  startHeight.value = store.terminalHeight
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value) return
  const delta = startY.value - e.clientY
  store.setTerminalHeight(startHeight.value + delta)
}

function stopResize() {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

onMounted(() => {
  store.connectWebSocket()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<template>
  <div class="h-screen flex flex-col">
    <!-- Header -->
    <header class="bg-panel-header border-b border-editor-border px-4 py-2 flex items-center gap-4">
      <h1 class="text-lg font-semibold text-white">FastC Playground</h1>
      <Toolbar />
      <div class="flex-1"></div>
      <a
        href="https://docs.skelfresearch.com/fastc"
        target="_blank"
        class="text-sm text-gray-400 hover:text-white transition-colors"
      >
        Documentation
      </a>
      <a
        href="https://github.com/Skelf-Research/fastc"
        target="_blank"
        class="text-sm text-gray-400 hover:text-white transition-colors"
      >
        GitHub
      </a>
    </header>

    <!-- Main content -->
    <main class="flex-1 flex flex-col bg-editor-border overflow-hidden">
      <!-- Top panels (editor + output) -->
      <div class="flex-1 grid grid-cols-2 gap-px overflow-hidden">
        <!-- FastC Editor -->
        <div class="bg-editor-bg flex flex-col">
          <div class="bg-panel-header px-3 py-1.5 text-xs uppercase text-gray-500 border-b border-editor-border flex items-center justify-between">
            <span>FastC Code</span>
            <span v-if="store.error" class="text-red-400 normal-case text-xs truncate max-w-xs" :title="store.error">
              {{ store.error }}
            </span>
          </div>
          <div class="flex-1 overflow-hidden">
            <Editor />
          </div>
        </div>

        <!-- Generated C Output -->
        <div class="bg-editor-bg flex flex-col">
          <div class="bg-panel-header px-3 py-1.5 text-xs uppercase text-gray-500 border-b border-editor-border">
            Generated C
          </div>
          <div class="flex-1 overflow-hidden">
            <OutputPanel />
          </div>
        </div>
      </div>

      <!-- Resize handle -->
      <div
        @mousedown="startResize"
        class="h-1 bg-editor-border hover:bg-blue-500 cursor-ns-resize transition-colors flex-shrink-0"
        :class="{ 'bg-blue-500': isResizing }"
      ></div>

      <!-- Terminal -->
      <div class="bg-[#0c0c0c] flex flex-col flex-shrink-0" :style="terminalStyle">
        <div class="bg-panel-header px-3 py-1.5 text-xs uppercase text-gray-500 border-b border-editor-border flex items-center justify-between">
          <span>Terminal</span>
          <button
            @click="store.clearTerminal()"
            class="text-xs text-gray-500 hover:text-white transition-colors"
          >
            Clear
          </button>
        </div>
        <div class="flex-1 overflow-hidden">
          <Terminal />
        </div>
      </div>
    </main>
  </div>
</template>
