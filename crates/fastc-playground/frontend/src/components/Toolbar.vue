<script setup lang="ts">
import { usePlaygroundStore } from '../stores/playground'

const store = usePlaygroundStore()
</script>

<template>
  <div class="flex items-center gap-2">
    <button
      @click="store.compile()"
      :disabled="store.isCompiling"
      class="px-4 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm font-medium rounded transition-colors"
    >
      {{ store.isCompiling ? 'Compiling...' : 'Compile' }}
    </button>
    <button
      @click="store.run()"
      :disabled="store.isRunning || store.isCompiling"
      class="px-4 py-1.5 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm font-medium rounded transition-colors"
    >
      {{ store.isRunning ? 'Running...' : 'Run' }}
    </button>
    <button
      @click="store.format()"
      class="px-4 py-1.5 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors"
    >
      Format
    </button>
    <span v-if="store.error" class="text-red-400 text-sm ml-2">
      {{ store.error }}
    </span>
  </div>
</template>
