import js from '@eslint/js';
import ts from 'typescript-eslint';
import vue from 'eslint-plugin-vue';

export default ts.config(
  {
    ignores: ['**/dist/**', '**/node_modules/**', '**/target/**', '**/auto-imports.d.ts', '**/components.d.ts'],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  // essential（错误级规则）；格式化交由 Prettier
  ...vue.configs['flat/essential'],
  {
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: { parser: ts.parser },
    },
    rules: {
      // 原型阶段允许单词组件名（页面文件名即路由名）
      'vue/multi-word-component-names': 'off',
    },
  },
);
