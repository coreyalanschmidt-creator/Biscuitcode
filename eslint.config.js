// eslint.config.js
// ESLint 9 flat config for BiscuitCode.
// Phase 2 deliverable — Phase 1 scaffolded the lint script but left no config.

import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';

export default [
  {
    ignores: [
      'node_modules/**',
      'src-tauri/**',
      'dist/**',
      'tests/ttft-bench.ts',
      '*.config.js',
      '*.config.ts',
      '*.config.cjs',
    ],
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
      globals: {
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        HTMLElement: 'readonly',
        CustomEvent: 'readonly',
        Event: 'readonly',
        EventTarget: 'readonly',
        KeyboardEvent: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        localStorage: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      // Allow unused vars prefixed with _ (common pattern for intentional ignores)
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      // Allow explicit any in a few places during early phases
      '@typescript-eslint/no-explicit-any': 'warn',
      // console usage: warn so intentional console calls can be suppressed
      // with eslint-disable-next-line no-console (see src/main.tsx).
      'no-console': 'warn',
    },
  },
];
