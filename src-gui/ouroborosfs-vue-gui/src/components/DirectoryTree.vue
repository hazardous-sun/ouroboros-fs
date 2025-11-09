<script setup lang="ts">
import {ref, onMounted, onUnmounted} from 'vue'
import fileListData from '../../file_structure.json'

const props = withDefaults(
    defineProps<{
      refreshInterval?: number
    }>(),
    {
      refreshInterval: 2000
    }
)

const fileList = ref<string[]>([])
const isLoading = ref(false)
const lastUpdated = ref<string>('')
let timerId: number | undefined = undefined

async function fetchData() {
  isLoading.value = true
  console.log('Refreshing file list from JSON...')
  fileList.value = fileListData.slice()
  const timestamp = new Date().toLocaleTimeString()
  lastUpdated.value = `Last Updated: ${timestamp}`
  isLoading.value = false
}

onMounted(() => {
  // 1. Fetch data on initial component load
  fetchData()

  // 2. Set up the auto-refresh timer
  timerId = window.setInterval(() => {
    fetchData()
  }, props.refreshInterval)
})

onUnmounted(() => {
  // 3. Clean up the timer when the component is destroyed
  if (timerId) {
    clearInterval(timerId)
  }
})
</script>

<template>
  <div class="directory-tree-container">
    <div class="header">
      <div>
        <h3>File List</h3>
        <small>{{ lastUpdated }}</small>
      </div>
      <button @click="fetchData" :disabled="isLoading">
        {{ isLoading ? 'Refreshing...' : 'Refresh' }}
      </button>
    </div>

    <div class="tree-content">
      <div v-if="isLoading && fileList.length === 0">Loading...</div>

      <ul v-else class="file-list">
        <li v-for="file in fileList" :key="file" class="file-item">
          {{ file }}
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.directory-tree-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 0 10px;
  min-width: 250px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #ccc;
  padding-bottom: 8px;
  flex-shrink: 0;
}

.header h3 {
  margin: 0;
}

.header small {
  font-size: 0.8em;
  color: #555;
}

.tree-content {
  flex-grow: 1;
  overflow-y: auto;
  padding-top: 10px;
}

.file-list {
  list-style: none;
  padding-left: 5px;
  margin: 0;
  font-family: monospace;
}

.file-item {
  padding: 4px 8px;
  border-radius: 4px;
}

.file-item:hover {
  background-color: #f0f0f0;
}
</style>