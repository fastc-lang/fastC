<script setup lang="ts">
import { usePlaygroundStore } from '../stores/playground'
import { computed, ref, watch, nextTick } from 'vue'

const store = usePlaygroundStore()
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const lineNumbersRef = ref<HTMLDivElement | null>(null)

const lines = computed(() => store.code.split('\n'))
const lineCount = computed(() => lines.value.length)

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+Enter to run
  if (e.ctrlKey && e.key === 'Enter') {
    e.preventDefault()
    store.run()
  }
  // Ctrl+S to compile
  if (e.ctrlKey && e.key === 's') {
    e.preventDefault()
    store.compile()
  }
  // Tab key handling
  if (e.key === 'Tab') {
    e.preventDefault()
    const textarea = e.target as HTMLTextAreaElement
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const value = textarea.value
    textarea.value = value.substring(0, start) + '    ' + value.substring(end)
    textarea.selectionStart = textarea.selectionEnd = start + 4
    store.updateCode(textarea.value)
  }
}

function handleInput(e: Event) {
  const textarea = e.target as HTMLTextAreaElement
  store.updateCode(textarea.value)
  store.clearError()
}

function handleScroll(e: Event) {
  const textarea = e.target as HTMLTextAreaElement
  if (lineNumbersRef.value) {
    lineNumbersRef.value.scrollTop = textarea.scrollTop
  }
}

function isErrorLine(lineNum: number): boolean {
  return store.errorLine === lineNum
}
</script>

<template>
  <div class="w-full h-full flex overflow-hidden">
    <!-- Line numbers -->
    <div
      ref="lineNumbersRef"
      class="flex-shrink-0 bg-editor-bg text-gray-500 font-mono text-sm leading-relaxed py-4 pr-2 text-right select-none overflow-hidden border-r border-editor-border"
      style="width: 48px;"
    >
      <div
        v-for="lineNum in lineCount"
        :key="lineNum"
        :class="[
          'px-2',
          isErrorLine(lineNum) ? 'bg-red-900/50 text-red-400' : ''
        ]"
      >
        {{ lineNum }}
      </div>
    </div>
    <!-- Code editor -->
    <div class="flex-1 relative">
      <!-- Error highlight overlay -->
      <div
        v-if="store.errorLine"
        class="absolute left-0 right-0 pointer-events-none font-mono text-sm leading-relaxed py-4"
      >
        <div
          v-for="lineNum in lineCount"
          :key="lineNum"
          :class="[
            'px-4',
            isErrorLine(lineNum) ? 'bg-red-900/30' : ''
          ]"
        >&nbsp;</div>
      </div>
      <textarea
        ref="textareaRef"
        :value="store.code"
        @input="handleInput"
        @keydown="handleKeydown"
        @scroll="handleScroll"
        class="w-full h-full bg-transparent text-gray-300 font-mono text-sm leading-relaxed p-4 resize-none outline-none relative z-10"
        spellcheck="false"
        placeholder="Enter FastC code here..."
      ></textarea>
    </div>
  </div>
</template>
