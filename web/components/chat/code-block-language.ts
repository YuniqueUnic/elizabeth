export const DEFAULT_CODE_BLOCK_LANGUAGE = "javascript";

export const CODE_BLOCK_LANGUAGE_OPTIONS = [
  { value: "javascript", label: "JavaScript" },
  { value: "typescript", label: "TypeScript" },
  { value: "tsx", label: "TSX" },
  { value: "jsx", label: "JSX" },
  { value: "rust", label: "Rust" },
  { value: "python", label: "Python" },
  { value: "go", label: "Go" },
  { value: "java", label: "Java" },
  { value: "c", label: "C" },
  { value: "cpp", label: "C++" },
  { value: "csharp", label: "C#" },
  { value: "ruby", label: "Ruby" },
  { value: "php", label: "PHP" },
  { value: "swift", label: "Swift" },
  { value: "kotlin", label: "Kotlin" },
  { value: "dart", label: "Dart" },
  { value: "bash", label: "Bash" },
  { value: "powershell", label: "PowerShell" },
  { value: "html", label: "HTML" },
  { value: "css", label: "CSS" },
  { value: "scss", label: "SCSS" },
  { value: "json", label: "JSON" },
  { value: "yaml", label: "YAML" },
  { value: "toml", label: "TOML" },
  { value: "sql", label: "SQL" },
  { value: "graphql", label: "GraphQL" },
  { value: "protobuf", label: "Protobuf" },
  { value: "dockerfile", label: "Dockerfile" },
  { value: "makefile", label: "Makefile" },
  { value: "xml", label: "XML" },
  { value: "markdown", label: "Markdown" },
  { value: "text", label: "Plain Text" },
] as const;

const CODE_BLOCK_LANGUAGE_ALIASES: Record<string, string> = {
  js: "javascript",
  ts: "typescript",
  py: "python",
  rb: "ruby",
  rs: "rust",
  "c++": "cpp",
  cpp: "cpp",
  "c#": "csharp",
  cs: "csharp",
  kt: "kotlin",
  kts: "kotlin",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  fish: "bash",
  ps1: "powershell",
  docker: "dockerfile",
  yml: "yaml",
  gql: "graphql",
  proto: "protobuf",
  md: "markdown",
  txt: "text",
  plaintext: "text",
};

export function normalizeCodeBlockLanguage(
  language: string | null | undefined,
  fallback = DEFAULT_CODE_BLOCK_LANGUAGE,
) {
  const normalized = language?.trim().toLowerCase();
  if (!normalized) return fallback;

  return CODE_BLOCK_LANGUAGE_ALIASES[normalized] || normalized;
}

export function getCodeBlockLanguageLabel(language: string) {
  const normalized = normalizeCodeBlockLanguage(language, "text");
  return CODE_BLOCK_LANGUAGE_OPTIONS.find((option) => option.value === normalized)
    ?.label ?? normalized;
}
