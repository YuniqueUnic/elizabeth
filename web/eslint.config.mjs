import nextCoreWebVitals from "eslint-config-next/core-web-vitals";
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
