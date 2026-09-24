// 本包被 Vite 应用以源码方式消费；此处仅声明用到的 env 字段，避免依赖 vite 类型包。
interface ImportMetaEnv {
  readonly VITE_API_BASE?: string;
  readonly VITE_WS_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
