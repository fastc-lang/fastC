<script setup lang="ts">
import { usePlaygroundStore } from '../stores/playground'

const store = usePlaygroundStore()
</script>

<template>
  <div class="w-full h-full overflow-auto p-4">
    <pre
      v-if="store.cCode"
      class="text-gray-300 font-mono text-sm leading-relaxed whitespace-pre-wrap"
    >{{ store.cCode }}</pre>
    <div v-else class="text-gray-500 text-sm italic">
      Generated C code will appear here after compilation.
    </div>
  </div>
</template>
