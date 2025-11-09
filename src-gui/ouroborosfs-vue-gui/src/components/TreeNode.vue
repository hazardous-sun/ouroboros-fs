<script setup lang="ts">
import { ref } from 'vue'

export interface Node {
  id: string
  name: string
  type: 'file' | 'folder'
  children?: Node[]
}

const props = defineProps<{
  node: Node
}>()

const isExpanded = ref(false)

function toggleExpand() {
  if (props.node.type === 'folder') {
    isExpanded.value = !isExpanded.value
  }
}
</script>

<template>
  <div class="tree-node">
    <div class="node-item" @click="toggleExpand">
      <span v-if="node.type === 'folder'" class="toggle">
        {{ isExpanded ? '[-]' : '[+]' }}
      </span>
      <span v-else class="toggle"></span>
      <span class="name">{{ node.name }}</span>
    </div>

    <div v-if="node.type === 'folder' && isExpanded" class="node-children">
      <TreeNode
          v-for="childNode in node.children"
          :key="childNode.id"
          :node="childNode"
      />
    </div>
  </div>
</template>

<style scoped>
.tree-node {
  font-family: monospace;
  font-size: 1.1em;
}

.node-item {
  cursor: pointer;
  user-select: none;
  padding: 2px 0;
}
.node-item:hover {
  background-color: #f0f0f0;
}

.toggle {
  display: inline-block;
  width: 20px;
  font-weight: bold;
}

.node-children {
  padding-left: 25px;
  border-left: 1px dashed #ccc;
  margin-left: 10px;
}
</style>