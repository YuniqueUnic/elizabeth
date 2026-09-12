import nextCoreWebVitals from "eslint-config-next/core-web-vitals";
import { parser } from "typescript-eslint";
import React from "react";

const config = [
  {
    ignores: [
      ".next/**",
      "node_modules/**",
      "types/generated/**",
      "playwright-report/**",
      "test-results/**",
      "target/**",
      "dev-assets/**",
    ],
  },
  ...nextCoreWebVitals,
  {
    // eslint-config-next/parser bundles a scope manager predating ESLint 10's
    // ScopeManager#addGlobals (vercel/next.js#89764); plain JS configs parse
    // fine with the ESLint-10-compatible typescript parser.
    files: ["**/*.js", "**/*.mjs", "**/*.cjs"],
    languageOptions: { parser },
  },
  {
    settings: {
      react: {
        version: React.version,
      },
    },
    rules: {
      "@next/next/no-html-link-for-pages": "off",
      "react-hooks/set-state-in-effect": "off",
    },
  },
  {
    files: ["e2e/**/*.ts"],
    rules: {
      "react-hooks/rules-of-hooks": "off",
    },
  },
  {
    files: ["components/ui/**/*.tsx"],
    rules: {
      "tailwindcss/classnames-order": "off",
      "tailwindcss/no-custom-classname": "off",
    },
  },
];

export default config;
