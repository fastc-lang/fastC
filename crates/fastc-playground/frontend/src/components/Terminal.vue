<script setup lang="ts">
import { usePlaygroundStore } from '../stores/playground'
import { computed, ref, watch, nextTick } from 'vue'

const store = usePlaygroundStore()
const terminalRef = ref<HTMLDivElement | null>(null)

// Convert ANSI codes to HTML
function ansiToHtml(text: string): string {
  return text
    .replace(/\x1b\[31m/g, '<span class="text-red-400">')
    .replace(/\x1b\[32m/g, '<span class="text-green-400">')
    .replace(/\x1b\[33m/g, '<span class="text-yellow-400">')
    .replace(/\x1b\[0m/g, '</span>')
    .replace(/\n/g, '<br>')
}

const formattedOutput = computed(() => {
  return store.terminalOutput.map(line => ansiToHtml(line)).join('')
})

// Auto-scroll to bottom
watch(() => store.terminalOutput.length, async () => {
  await nextTick()
  if (terminalRef.value) {
    terminalRef.value.scrollTop = terminalRef.value.scrollHeight
  }
})
</script>

<template>
  <div
    ref="terminalRef"
    class="w-full h-full overflow-auto p-4 font-mono text-sm text-green-400 leading-relaxed"
  >
    <div v-if="formattedOutput" v-html="formattedOutput"></div>
    <div v-else class="text-gray-500 italic">
      Terminal output will appear here when you run your code.
      <br><br>
      Keyboard shortcuts:
      <br>
      • Ctrl+Enter: Run
      <br>
      • Ctrl+S: Compile
    </div>
  </div>
</template>
