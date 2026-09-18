#!/usr/bin/env node
/**
 * i18n 死键审计：找出 `web/messages/**` 中永远取不到的文案键。
 *
 * 判定分三层，置信度递减，报告里分开列出：
 *
 *   1. 确认死键 —— 键的叶名在整个源码树里**没有作为字符串字面量出现过**。
 *      纯静态可达性判定的结果，零误报，可直接删。
 *   2. 待确认 —— 叶名在别处出现过（通常是**另一个命名空间**的同名键）。
 *      报告给出 file:line，人工扫一眼即可判定。典型误报形态：
 *      `common.passwordLabel` 已废弃，真实调用是
 *      `room.passwordDialog.passwordLabel` / `room.closeRoom.passwordLabel`。
 *   3. 推断死键 —— `t(`pre.${x}`)` 这类动态键，静态分析无法穷举 `x`。
 *      当 `x` 来自代码里的有限枚举时，用取值域与 JSON 求差。
 *      **取值域会随代码演进过期**，改动相关枚举后务必复核（见 DOMAIN_RULES）。
 *
 * 用法：
 *   bun run i18n:audit            只报告，始终退出 0
 *   bun run i18n:audit --strict   发现任何死键就退出 1（可挂 CI）
 *
 * 已知盲区（会在报告末尾告警）：
 *   - `useTranslations()` 无参调用、非字面量命名空间参数、`getTranslations({...})`
 *   - 解构式声明 `const { t: x } = useTranslations(...)`
 *   - `t(变量)` 这种键名本身不是字面量的调用
 *   - 把 `t` 当参数传给**别的文件**（同文件内传递被文件级归属覆盖）
 */

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const WEB_DIR = fileURLToPath(new URL("..", import.meta.url));
const MESSAGES_DIR = path.join(WEB_DIR, "messages");
const PRIMARY_LOCALE = "zh";
const SOURCE_EXTENSIONS = new Set([".ts", ".tsx"]);
const IGNORED_DIRS = new Set([
  "node_modules",
  ".next",
  "out",
  "web-out",
  "target",
  "playwright-report",
  "test-results",
  "types",
  "dev-assets",
]);

/**
 * 动态前缀的取值域规则。
 *
 * 只有当 `t(`pre.${x}`)` 的 `x` 来自代码里的**有限枚举**时才可推断。
 * 每条规则都要写清取值域的来源 —— 代码演进后这里必须同步复核，
 * 否则会把可达键判成死键（误报）。报告里为此单列一节。
 */
const DOMAIN_RULES = [
  {
    ns: "common",
    prefix: "desktopNotification.fallback.",
    // 来源：lib/desktop-notifications.ts 的 ContentDesktopNotificationKind
    allowed: ["message", "file", "link"],
    source: "lib/desktop-notifications.ts → ContentDesktopNotificationKind",
  },
  {
    ns: "common",
    prefix: "desktopNotification.roomUpdateSubject.",
    // RoomClientPage.tsx 用三元把 address_changed 分流到带 {path} 的 addressChanged，
    // 所以 `${action}` 只会产出 roles_changed / settings_changed。
    allowed: ["addressChanged", "roles_changed", "settings_changed"],
    source: "app/[roomName]/RoomClientPage.tsx → notifyRoomUpdate",
  },
  {
    ns: "common",
    prefix: "desktopNotification.summary.room.",
    // 后端 RoomUpdateReason 是 snake_case，只有三个变体
    allowed: ["address_changed", "roles_changed", "settings_changed"],
    source: "crates/board/src/websocket/types.rs → RoomUpdateReason",
  },
  {
    ns: "common",
    prefix: "desktopNotification.title.room.",
    allowed: ["address_changed", "roles_changed", "settings_changed"],
    source: "crates/board/src/websocket/types.rs → RoomUpdateReason",
  },
  {
    ns: "settings",
    prefix: "desktopNotifications.actions.",
    // desktopNotificationActionsByKind 的并集
    allowed: [
      "created",
      "updated",
      "deleted",
      "address_changed",
      "roles_changed",
      "settings_changed",
    ],
    source: "lib/desktop-notifications.ts → desktopNotificationActionsByKind",
  },
];

const TRANSLATOR_DECL_RE =
  /(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:await\s+)?(?:useTranslations|getTranslations)\(\s*(?:"([^"]*)"|'([^']*)')?\s*\)/g;
const MESSAGES_IMPORT_RE =
  /import\s+([A-Za-z_$][\w$]*)\s+from\s+["'][^"']*\/messages\/([^/"']+)\/([^/"']+)\.json["']/g;
const QUOTED_SPAN_RE = /(["'`])([\s\S]*?)\1/g;

/** 语句边界：行首开始的一条顶层声明 */
const STATEMENT_BOUNDARY_RE = /(?:^|\n)(?=\s*(?:export\s+)?(?:const|let|var|function|class|interface|type)\b)/g;
/** 形如 `export const tRoom = (key, values?) => translate(zhRoom, key, values)` 的包装函数 */
const KEY_WRAPPER_HEAD_RE = /^\s*(export\s+)?(?:const|let|var|function)\s+([A-Za-z_$][\w$]*)\s*(?:=\s*)?\(\s*key\b/;

/** 这些形态不在解析范围内，出现即告警 */
const BLIND_SPOTS = [
  [/(?:useTranslations|getTranslations)\s*\(\s*\)/, "useTranslations() 无参调用"],
  [/(?:const|let)\s*\{[^}]*\}\s*=\s*(?:await\s+)?(?:useTranslations|getTranslations)\s*\(/, "解构式 translator 声明"],
  [/(?:useTranslations|getTranslations)\s*\(\s*\{/, "getTranslations({...}) 对象参数"],
  [/[A-Za-z_$][\w$]*\s*:\s*(?:ReturnType<typeof useTranslations>|Translate)\b/, "t 作为参数传递"],
];

const escapeRe = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const relative = (file) => path.relative(WEB_DIR, file);
const lineOf = (text, index) => text.slice(0, index).split("\n").length;

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!IGNORED_DIRS.has(entry.name)) walk(path.join(dir, entry.name), out);
    } else if (SOURCE_EXTENSIONS.has(path.extname(entry.name))) {
      out.push(path.join(dir, entry.name));
    }
  }
  return out;
}

const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));

function leafPaths(node, prefix = "", out = []) {
  for (const [key, value] of Object.entries(node)) {
    const dotted = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object") leafPaths(value, dotted, out);
    else out.push(dotted);
  }
  return out;
}

const listDirs = (dir) =>
  fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();

const listNamespaces = (locale) =>
  fs
    .readdirSync(path.join(MESSAGES_DIR, locale))
    .filter((name) => name.endsWith(".json"))
    .map((name) => path.basename(name, ".json"))
    .sort();

/**
 * 识别把 bundle 包成"取键函数"的包装器：
 *   export const tRoom = (key, values?) => translate(zhRoom as MessageBundle, key, values)
 * 这类函数的第一个参数就是相对 bundle 根的键，等价于直接调用 bundle。
 * 逐"语句块"判定，避免把相邻声明的 bundle 张冠李戴。
 * 带 `export` 的包装器会被其它文件 import 后调用（e2e 的 tRoom/tAdmin 就是），
 * 所以单独收集，供全局生效。
 */
function collectKeyWrappers(text, bundle) {
  const wrappers = [];
  for (const chunk of text.split(STATEMENT_BOUNDARY_RE)) {
    const head = KEY_WRAPPER_HEAD_RE.exec(chunk);
    if (!head) continue;
    const bundleAt = chunk.indexOf(bundle.variable);
    if (bundleAt < 0) continue;
    // bundle 之后还要再出现 key，才说明它被当作键传进去了
    if (!/\bkey\b/.test(chunk.slice(bundleAt + bundle.variable.length))) continue;
    wrappers.push({
      variable: head[2],
      nsPath: bundle.ns,
      rootRelative: true,
      exported: Boolean(head[1]),
    });
  }
  return wrappers;
}

/**
 * 每个源文件的 translator 绑定。
 * `useTranslations("room.config")` → 前缀 room.config；
 * `import zhRoom from ".../messages/zh/room.json"` → 前缀 room，键是相对 bundle 根的完整点号路径；
 * 包装器（tRoom 之类）同样按相对 bundle 根处理，导出的包装器全局可用。
 */
function collectBindings(files, texts) {
  const perFile = new Map();
  const exportedWrappers = new Map();
  const anomalies = [];

  for (const file of files) {
    const text = texts.get(file);
    const bindings = [];
    const bundles = [];

    TRANSLATOR_DECL_RE.lastIndex = 0;
    for (let m = TRANSLATOR_DECL_RE.exec(text); m; m = TRANSLATOR_DECL_RE.exec(text)) {
      const nsPath = m[2] ?? m[3];
      if (nsPath !== undefined) bindings.push({ variable: m[1], nsPath, rootRelative: false });
    }

    MESSAGES_IMPORT_RE.lastIndex = 0;
    for (let m = MESSAGES_IMPORT_RE.exec(text); m; m = MESSAGES_IMPORT_RE.exec(text)) {
      if (m[2] !== PRIMARY_LOCALE) continue;
      bundles.push({ variable: m[1], ns: m[3] });
      bindings.push({ variable: m[1], nsPath: m[3], rootRelative: true });
    }
    for (const bundle of bundles) {
      for (const wrapper of collectKeyWrappers(text, bundle)) {
        bindings.push(wrapper);
        if (wrapper.exported) exportedWrappers.set(wrapper.variable, wrapper.nsPath);
      }
    }

    if (bindings.length > 0) perFile.set(file, bindings);

    // "t 作参数"只在自己没有 translator 声明的文件里才是真盲区
    if (bindings.length === 0) {
      for (const [re, label] of BLIND_SPOTS) {
        const m = re.exec(text);
        if (m) anomalies.push(`${relative(file)}:${lineOf(text, m.index)} ${label}`);
      }
    }
  }

  return { perFile, exportedWrappers, anomalies };
}

function collectUsage(files, perFile, exportedWrappers, texts, namespaces) {
  const known = new Set(namespaces);
  const reachable = new Map();
  const dynamic = new Map();
  const consumers = new Map();
  const dynamicSites = new Map();
  const emptyPrefixes = [];

  const add = (map, ns, value) => {
    if (!map.has(ns)) map.set(ns, new Set());
    map.get(ns).add(value);
  };

  for (const file of files) {
    // 导出的包装器（tRoom/tAdmin/...）在所有文件里都可能是取键入口
    const bindings = [
      ...(perFile.get(file) ?? []),
      ...[...exportedWrappers].map(([variable, nsPath]) => ({
        variable,
        nsPath,
        rootRelative: true,
      })),
    ];
    if (bindings.length === 0) continue;
    const text = texts.get(file);

    for (const { variable, nsPath, rootRelative } of bindings) {
      const root = nsPath.split(".")[0];
      if (!known.has(root)) continue;
      const sub = rootRelative ? "" : nsPath.slice(root.length + 1);
      if (!rootRelative) add(consumers, root, relative(file));

      const patterns = [
        new RegExp(`\\b${escapeRe(variable)}\\s*\\(\\s*(["'\`])([\\s\\S]*?)\\1`, "g"),
        new RegExp(
          `\\b${escapeRe(variable)}\\.(?:rich|raw|markup|has)\\s*\\(\\s*(["'\`])([\\s\\S]*?)\\1`,
          "g",
        ),
      ];

      for (const pattern of patterns) {
        for (let m = pattern.exec(text); m; m = pattern.exec(text)) {
          const key = m[2];
          const marker = key.indexOf("${");
          if (marker >= 0) {
            const staticPrefix = key.slice(0, marker);
            // 空静态前缀（`t(`${x}`)`）无法收敛到任何键，是硬盲区
            if (!staticPrefix) {
              emptyPrefixes.push(`${relative(file)}:${lineOf(text, m.index)} ${variable}(…${key})`);
            }
            const prefix = sub ? `${sub}.${staticPrefix}` : staticPrefix;
            add(dynamic, root, prefix);
            if (!dynamicSites.has(root)) dynamicSites.set(root, new Map());
            const sites = dynamicSites.get(root);
            if (!sites.has(prefix)) sites.set(prefix, []);
            sites.get(prefix).push(`${relative(file)}:${lineOf(text, m.index)}`);
          } else {
            add(reachable, root, sub ? `${sub}.${key}` : key);
          }
        }
      }
    }
  }

  return { reachable, dynamic, consumers, dynamicSites, emptyPrefixes };
}

/** 全量字符串字面量：叶名 -> [file:line]，用于"待确认"定位 */
function collectLiterals(files, texts) {
  const byLeaf = new Map();
  for (const file of files) {
    const text = texts.get(file);
    QUOTED_SPAN_RE.lastIndex = 0;
    for (let m = QUOTED_SPAN_RE.exec(text); m; m = QUOTED_SPAN_RE.exec(text)) {
      const raw = m[2];
      if (raw.includes("\n")) continue;
      const marker = raw.indexOf("${");
      const literal = marker >= 0 ? raw.slice(0, marker) : raw;
      if (!literal) continue;
      const leaf = literal.split(".").pop();
      if (!leaf) continue;
      if (!byLeaf.has(leaf)) byLeaf.set(leaf, []);
      const hits = byLeaf.get(leaf);
      const site = `${relative(file)}:${lineOf(text, m.index)}`;
      if (!hits.includes(site)) hits.push(site);
    }
  }
  return byLeaf;
}

function classify(namespaces, keysByNs, usage, literals) {
  const confirmed = [];
  const suspects = [];
  const inferred = [];

  for (const ns of namespaces) {
    const reachable = usage.reachable.get(ns) ?? new Set();
    const prefixes = [...(usage.dynamic.get(ns) ?? new Set())].filter(Boolean);

    for (const key of keysByNs.get(ns)) {
      if (reachable.has(key)) continue;

      if (prefixes.some((prefix) => key.startsWith(prefix))) {
        const rule = DOMAIN_RULES.find(
          (candidate) => candidate.ns === ns && key.startsWith(candidate.prefix),
        );
        if (rule && !rule.allowed.includes(key.slice(rule.prefix.length))) {
          inferred.push({ ns, key, rule, sites: usage.dynamicSites.get(ns)?.get(rule.prefix) ?? [] });
        }
        continue;
      }

      const hits = literals.get(key.split(".").pop()) ?? [];
      if (hits.length > 0) suspects.push({ ns, key, hits });
      else confirmed.push({ ns, key });
    }
  }

  return { confirmed, suspects, inferred };
}

function printSection(title, note, items, render) {
  console.log(`\n${title}（${items.length}）`);
  if (note) console.log(`  ${note}`);
  if (items.length === 0) {
    console.log("  无");
    return;
  }
  for (const item of items) render(item);
}

function main() {
  const strict = process.argv.includes("--strict");

  if (!fs.existsSync(MESSAGES_DIR)) {
    console.error(`找不到 ${relative(MESSAGES_DIR)}`);
    process.exit(2);
  }

  const locales = listDirs(MESSAGES_DIR);
  if (!locales.includes(PRIMARY_LOCALE)) {
    console.error(`缺少主语言目录 messages/${PRIMARY_LOCALE}`);
    process.exit(2);
  }

  const namespaces = listNamespaces(PRIMARY_LOCALE);
  const keysByNs = new Map(
    namespaces.map((ns) => [ns, leafPaths(readJson(path.join(MESSAGES_DIR, PRIMARY_LOCALE, `${ns}.json`)))]),
  );

  const files = walk(WEB_DIR);
  const texts = new Map(files.map((file) => [file, fs.readFileSync(file, "utf8")]));
  const { perFile, exportedWrappers, anomalies } = collectBindings(files, texts);
  const usage = collectUsage(files, perFile, exportedWrappers, texts, namespaces);
  const literals = collectLiterals(files, texts);
  const { confirmed, suspects, inferred } = classify(namespaces, keysByNs, usage, literals);

  console.log(`i18n 死键审计  主语言 messages/${PRIMARY_LOCALE}  扫描 ${files.length} 个源文件`);

  const countIn = (list, ns) => list.filter((item) => item.ns === ns).length;
  console.log(
    `\n${"命名空间".padEnd(12)}${"叶子".padStart(7)}${"消费者".padStart(8)}${"动态前缀".padStart(10)}${"确认死键".padStart(10)}${"待确认".padStart(8)}${"推断死键".padStart(10)}`,
  );
  for (const ns of namespaces) {
    console.log(
      ns.padEnd(12) +
        String(keysByNs.get(ns).length).padStart(7) +
        String((usage.consumers.get(ns) ?? new Set()).size).padStart(8) +
        String((usage.dynamic.get(ns) ?? new Set()).size).padStart(10) +
        String(countIn(confirmed, ns)).padStart(10) +
        String(countIn(suspects, ns)).padStart(8) +
        String(countIn(inferred, ns)).padStart(10),
    );
  }

  printSection("确认死键", "叶名从未作为字符串字面量出现 —— 可直接删", confirmed, (item) => {
    console.log(`  ${item.ns}.json  ${item.key}`);
  });

  printSection(
    "待确认",
    "叶名在别处出现，通常属于另一个命名空间；按 file:line 扫一眼即可判定",
    suspects,
    (item) => {
      console.log(`  ${item.ns}.json  ${item.key}`);
      for (const hit of item.hits.slice(0, 4)) console.log(`        ${hit}`);
      if (item.hits.length > 4) console.log(`        ...另有 ${item.hits.length - 4} 处`);
    },
  );

  printSection(
    "推断死键",
    "动态前缀取值域比对 —— 取值域过期会误报，改动相关枚举后请复核",
    inferred,
    (item) => {
      console.log(`  ${item.ns}.json  ${item.key}`);
      console.log(`        规则 ${item.rule.prefix} → {${item.rule.allowed.join(", ")}}`);
      console.log(`        来源 ${item.rule.source}`);
      for (const site of item.sites.slice(0, 3)) console.log(`        调用点 ${site}`);
    },
  );

  const dynamicRows = [];
  for (const ns of namespaces) {
    for (const [prefix, sites] of usage.dynamicSites.get(ns) ?? []) {
      dynamicRows.push({ ns, prefix, sites });
    }
  }
  printSection("动态前缀清单", "自动化判断最弱的地方：改动相关代码后请复核取值域", dynamicRows, (row) => {
    const rule = DOMAIN_RULES.find(
      (candidate) => candidate.ns === row.ns && candidate.prefix === row.prefix,
    );
    console.log(`  ${row.ns}.json  ${row.prefix}\${…}  ${rule ? "有取值域规则" : "无规则，整体视为可达"}`);
    for (const site of row.sites.slice(0, 3)) console.log(`        ${site}`);
  });

  let mismatch = 0;
  for (const locale of locales.filter((name) => name !== PRIMARY_LOCALE)) {
    for (const ns of namespaces) {
      const mine = new Set(keysByNs.get(ns));
      const theirs = new Set(leafPaths(readJson(path.join(MESSAGES_DIR, locale, `${ns}.json`))));
      const onlyMine = [...mine].filter((key) => !theirs.has(key));
      const onlyTheirs = [...theirs].filter((key) => !mine.has(key));
      if (onlyMine.length + onlyTheirs.length === 0) continue;
      mismatch += onlyMine.length + onlyTheirs.length;
      console.log(
        `\n键集合不一致：messages/${locale}/${ns}.json 与主语言差 ${onlyMine.length + onlyTheirs.length} 个` +
          (onlyMine.length ? `\n  仅主语言有：${onlyMine.slice(0, 6).join(", ")}` : "") +
          (onlyTheirs.length ? `\n  仅该语言有：${onlyTheirs.slice(0, 6).join(", ")}` : ""),
      );
    }
  }
  console.log(
    mismatch === 0
      ? `\n语言间键集合一致：${locales.join(" / ")}`
      : `\n语言间键集合差异合计 ${mismatch} 个`,
  );

  if (usage.emptyPrefixes.length > 0) {
    console.log(`\n分析盲区告警（${usage.emptyPrefixes.length}）—— 空静态前缀的动态键，无法收敛到任何键`);
    for (const site of usage.emptyPrefixes) console.log(`  ${site}`);
  }

  if (anomalies.length > 0) {
    console.log(`\n分析盲区告警（${anomalies.length}）—— 这些形态不在解析范围内，结论可能失真`);
    for (const anomaly of anomalies) console.log(`  ${anomaly}`);
  }

  const dead = confirmed.length + inferred.length;
  console.log(
    `\n确认死键 ${confirmed.length} + 推断死键 ${inferred.length} = ${dead}` +
      `（另 ${suspects.length} 个待人工确认）`,
  );

  if (strict && (dead > 0 || suspects.length > 0 || mismatch > 0 || anomalies.length > 0 || usage.emptyPrefixes.length > 0)) {
    process.exit(1);
  }
}

main();
