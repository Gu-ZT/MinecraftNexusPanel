/** 当前选中 Core：切换驱动实例/环境/镜像等节点级数据源。 */

import { ref } from 'vue';
import { defineStore } from 'pinia';

export const useCoreStore = defineStore('core', () => {
  /** null = 全部节点（实例列表等聚合视图）。 */
  const selectedCoreId = ref<string | null>(null);

  function select(coreId: string | null): void {
    selectedCoreId.value = coreId;
  }

  return { selectedCoreId, select };
});
