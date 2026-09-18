#!/usr/bin/env bun
/**
 * e2e 死代码审计（声明级可达性）
 *
 * 为什么不能只看"名字有没有出现"：删掉一个未使用的 task 后，它消费的 interaction 会变死，
 * 再往下它消费的页面对象定位器也会变死。这是**级联**，rg 查不出来。本脚本做真正的传递分析。
 *
 * 分析分两层：
 *   1. 模块层 —— 入口是 `e2e/specs/**`，沿 import 边求 live 模块集。
 *   2. 声明层 —— 在 live 模块内求 live 顶层声明：
 *        · 种子：spec 模块的所有声明与模块级语句
 *        · 传播：live 声明引用了同文件的其它声明 / 引用了 import 进来的名字（→ 目标模块的对应声明）
 *      模块级语句（副作用）只要模块是 live 就算 live。
 *
 * 关键点：**某个定位器只有在"活代码"里被引用才算活**。
 * 若只看"被别的文件引用"，`RoomScreen.permissionButton`（仅被死的 `SetPermissionState` 引用）
 * 会被误判为活 —— 这正是第一版的假阴性。
 *
 * 用法：
 *   bun run e2e:audit            # 报告，始终退出 0
 *   bun run e2e:audit --strict   # 有任何发现就退出 1（可挂 CI）
 */

import fs from "node:fs";
import path from "node:path";
import ts from "typescript";

const WEB_DIR = path.resolve(import.meta.dir, "..");
const E2E_DIR = path.join(WEB_DIR, "e2e");
const SPECS_DIR = path.join(E2E_DIR, "specs");
const STRICT = process.argv.includes("--strict");
const rel = (p) => path.relative(WEB_DIR, p);

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(full));
    else if (entry.name.endsWith(".ts")) out.push(full);
  }
  return out;
}

function resolveSpecifier(specifier, fromFile, known) {
  if (!specifier.startsWith(".")) return undefined;
  const base = path.resolve(path.dirname(fromFile), specifier);
  for (const candidate of [`${base}.ts`, path.join(base, "index.ts")]) {
    if (known.has(candidate)) return candidate;
  }
  return undefined;
}

/** 从 VariableStatement 的绑定模式里取所有被声明的名字（含解构） */
function declaredNames(name, into) {
  if (ts.isIdentifier(name)) into.push(name.text);
  else if (ts.isObjectBindingPattern(name) || ts.isArrayBindingPattern(name)) {
    for (const element of name.elements) {
      if (ts.isBindingElement(element)) declaredNames(element.name, into);
    }
  }
}

const files = walk(E2E_DIR);
const known = new Set(files);
const modules = new Map();

for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);

  /** name -> { node, exported } */
  const decls = new Map();
  /** 模块级（非声明）语句 —— 副作用代码 */
  const sideEffects = [];
  /** 本文件导入的本地名 -> 目标模块 */
  const importedFrom = new Map();
  /** 目标模块 -> 从它导入的本地名集合 */
  const importStatements = [];
  /** export { ... } 的本地名（含 `export { x } from "./y"`） */
  const reexportedLocals = new Set();

  const addDecl = (name, node, exported) => {
    if (!name) return;
    decls.set(name, { node, exported });
  };

  const isExported = (node) =>
    (ts.getModifiers(node) ?? []).some((m) => m.kind === ts.SyntaxKind.ExportKeyword);

  for (const stmt of sf.statements) {
    if (ts.isImportDeclaration(stmt)) {
      const clause = stmt.importClause;
      const specifier = stmt.moduleSpecifier.text;
      const resolved = resolveSpecifier(specifier, file, known);
      const names = [];
      if (clause?.namedBindings && ts.isNamedImports(clause.namedBindings)) {
        for (const element of clause.namedBindings.elements) names.push(element.name.text);
      }
      if (clause?.name) names.push(clause.name.text);
      for (const n of names) importedFrom.set(n, resolved);
      importStatements.push({ node: stmt, resolved, names: new Set(names), specifier });
      continue;
    }
    if (ts.isExportDeclaration(stmt)) {
      if (stmt.moduleSpecifier) {
        // export { x } from "./y" —— 转发，不产生本地声明
        importStatements.push({
          node: stmt,
          resolved: resolveSpecifier(stmt.moduleSpecifier.text, file, known),
          names: new Set(),
          specifier: stmt.moduleSpecifier.text,
          forwardOnly: true,
        });
      } else if (stmt.exportClause && ts.isNamedExports(stmt.exportClause)) {
        for (const element of stmt.exportClause.elements) reexportedLocals.add(element.name.text);
      }
      sideEffects.push(stmt);
      continue;
    }
    if (ts.isVariableStatement(stmt)) {
      const names = [];
      for (const declaration of stmt.declarationList.declarations) declaredNames(declaration.name, names);
      for (const n of names) addDecl(n, stmt, isExported(stmt));
      continue;
    }
    if (
      ts.isFunctionDeclaration(stmt) ||
      ts.isClassDeclaration(stmt) ||
      ts.isInterfaceDeclaration(stmt) ||
      ts.isTypeAliasDeclaration(stmt) ||
      ts.isEnumDeclaration(stmt)
    ) {
      addDecl(stmt.name?.text, stmt, isExported(stmt));
      continue;
    }
    sideEffects.push(stmt);
  }

  modules.set(file, { file, text, sf, decls, sideEffects, importedFrom, importStatements, reexportedLocals });
}

// ---------- 第 1 层：模块可达性 ----------
const isSpec = (file) => file.startsWith(SPECS_DIR + path.sep);
const liveModules = new Set(files.filter(isSpec));
for (;;) {
  const next = [];
  for (const file of liveModules) {
    for (const { resolved } of modules.get(file).importStatements) {
      if (resolved && !liveModules.has(resolved)) next.push(resolved);
    }
  }
  if (next.length === 0) break;
  for (const f of next) liveModules.add(f);
}

// ---------- 第 2 层：声明可达性 ----------
const key = (file, name) => `${file}\u0000${name}`;
const liveDecls = new Set();
const liveSideEffects = new Set(liveModules); // 模块级语句：模块 live 即 live

const sameFileRefs = (node, ownName, decls) => {
  const text = node.getText(modules.get(node.getSourceFile().fileName)?.sf ?? node.getSourceFile());
  const hits = new Set();
  for (const [name, decl] of decls) {
    if (name === ownName) continue;
    if (new RegExp(`\\b${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`).test(text)) hits.add(name);
  }
  return hits;
};

const importedRefs = (node, file) => {
  const mod = modules.get(file);
  const sf = mod.sf;
  const text = node.getText(sf);
  const hits = new Set();
  for (const [local, target] of mod.importedFrom) {
    if (target && new RegExp(`\\b${local.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`).test(text)) {
      hits.add(key(target, local));
    }
  }
  return hits;
};

// 种子：spec 模块的所有声明
for (const file of liveModules) {
  if (!isSpec(file)) continue;
  for (const name of modules.get(file).decls.keys()) liveDecls.add(key(file, name));
}

// 种子：被 spec 之外的 live 声明 import 的导出 —— 需要先有 live 声明才能判断，故统一放进传播循环
const exportedByModule = new Map();
for (const [file, mod] of modules) {
  for (const [name, decl] of mod.decls) {
    if (decl.exported || mod.reexportedLocals.has(name)) {
      if (!exportedByModule.has(file)) exportedByModule.set(file, new Set());
      exportedByModule.get(file).add(name);
    }
  }
}

for (;;) {
  let grew = false;
  const add = (file, name) => {
    if (!modules.get(file)?.decls.has(name)) return;
    const k = key(file, name);
    if (!liveDecls.has(k)) {
      liveDecls.add(k);
      grew = true;
    }
  };

  for (const file of liveModules) {
    const mod = modules.get(file);
    // 模块级语句引用的本地声明 → 也是活的
    for (const stmt of mod.sideEffects) {
      for (const name of sameFileRefs(stmt, null, mod.decls)) add(file, name);
      for (const k of importedRefs(stmt, file)) add(...k.split("\u0000"));
    }
    // live 声明 → 同文件声明 + 跨模块导入
    for (const [name, decl] of mod.decls) {
      if (!liveDecls.has(key(file, name))) continue;
      for (const ref of sameFileRefs(decl.node, name, mod.decls)) add(file, ref);
      for (const k of importedRefs(decl.node, file)) add(...k.split("\u0000"));
    }
  }
  if (!grew) break;
}

/** 某位置是否落在"活代码"里 */
function isInLiveCode(file, position) {
  const mod = modules.get(file);
  if (isSpec(file)) return true;
  if (!liveModules.has(file)) return false;
  for (const [name, decl] of mod.decls) {
    if (position >= decl.node.getFullStart() && position < decl.node.getEnd()) {
      return liveDecls.has(key(file, name));
    }
  }
  for (const stmt of mod.sideEffects) {
    if (position >= stmt.getFullStart() && position < stmt.getEnd()) return liveSideEffects.has(file);
  }
  return false;
}

// ---------- 报告 ----------
const unreachableModules = files.filter((f) => !liveModules.has(f)).sort();

const unusedImports = [];
for (const file of [...liveModules].sort()) {
  const mod = modules.get(file);
  const liveText = [];
  for (const [name, decl] of mod.decls) {
    if (liveDecls.has(key(file, name))) liveText.push(decl.node.getText(mod.sf));
  }
  for (const stmt of mod.sideEffects) liveText.push(stmt.getText(mod.sf));
  const haystack = liveText.join("\n");
  for (const { names, specifier } of mod.importStatements) {
    for (const name of names) {
      const re = new RegExp(`\\b${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`);
      if (!re.test(haystack)) unusedImports.push({ file, name, specifier });
    }
  }
}

const deadExports = [];
for (const file of [...liveModules].sort()) {
  if (isSpec(file)) continue;
  for (const name of exportedByModule.get(file) ?? []) {
    if (!liveDecls.has(key(file, name))) deadExports.push({ file, name });
  }
}
deadExports.sort((a, b) => a.file.localeCompare(b.file) || a.name.localeCompare(b.name));
unusedImports.sort((a, b) => a.file.localeCompare(b.file) || a.name.localeCompare(b.name));

/** 页面对象：定位器只有在活代码里被引用才算活 */
const screenFindings = [];
for (const file of files) {
  const mod = modules.get(file);
  const objectName = [...mod.decls.keys()].find((name) => {
    const node = mod.decls.get(name).node;
    return ts.isVariableStatement(node) &&
      node.declarationList.declarations.some((d) => d.initializer && ts.isObjectLiteralExpression(d.initializer));
  });
  if (!objectName) continue;

  const stmt = mod.decls.get(objectName).node;
  const objectLiteral = stmt.declarationList.declarations
    .map((d) => d.initializer)
    .find((init) => init && ts.isObjectLiteralExpression(init));
  const props = new Map();
  for (const property of objectLiteral.properties) {
    if (ts.isPropertyAssignment(property) && ts.isIdentifier(property.name)) {
      props.set(property.name.text, property);
    }
  }
  if (props.size === 0) continue;

  const keys = [...props.keys()];
  const refRe = (k) => new RegExp(`\\b${objectName}\\.${k.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`);

  const roots = new Set();
  for (const other of files) {
    const otherMod = modules.get(other);
    for (const k of keys) {
      const re = refRe(k);
      for (const m of otherMod.text.matchAll(new RegExp(re.source, "g"))) {
        // 自身定义处（`k: (...) =>` 的属性名）不会匹配 `Obj.k`，故无需排除
        if (isInLiveCode(other, m.index)) {
          roots.add(k);
          break;
        }
      }
    }
  }

  const intra = new Map();
  for (const k of keys) {
    intra.set(k, new Set(keys.filter((o) => o !== k && refRe(o).test(props.get(k).getText(mod.sf)))));
  }
  for (;;) {
    let grew = false;
    for (const k of [...roots]) {
      for (const o of intra.get(k)) {
        if (!roots.has(o)) {
          roots.add(o);
          grew = true;
        }
      }
    }
    if (!grew) break;
  }

  const dead = keys.filter((k) => !roots.has(k));
  if (dead.length > 0) screenFindings.push({ file, objectName, total: keys.length, dead });
}

const specCount = files.filter(isSpec).length;
const total =
  unreachableModules.length + unusedImports.length + deadExports.length +
  screenFindings.reduce((sum, f) => sum + f.dead.length, 0);

console.log(`e2e 死代码审计：${files.length} 个模块，入口 spec ${specCount} 个，可达 ${liveModules.size} 个，活跃声明 ${liveDecls.size} 个\n`);

console.log(`不可达模块（${unreachableModules.length}）—— 没有任何 spec 路径能到达`);
for (const f of unreachableModules) console.log(`  ${rel(f)}`);

console.log(`\n未使用 import（${unusedImports.length}）—— 在活代码里从未出现`);
for (const { file, name, specifier } of unusedImports) console.log(`  ${rel(file)}  ${name} ← ${specifier}`);

console.log(`\n死导出（${deadExports.length}）—— 没有任何活代码引用`);
for (const { file, name } of deadExports) console.log(`  ${rel(file)}  ${name}`);

console.log(`\n页面对象定位器（${screenFindings.length} 个对象有待清理项）`);
for (const { file, objectName, total: count, dead } of screenFindings) {
  console.log(`  ${rel(file)}  ${objectName}（${count} 个定位器，未引用 ${dead.length}）`);
  for (const k of dead) console.log(`      ${k}`);
}

if (total === 0) {
  console.log("\n结论：无发现。");
} else {
  console.log(`\n结论：共 ${total} 项。删除前请确认：`);
  console.log("  · 定位器指向的 testid 是否在生产代码里仍存在（删的是未使用的 page-object API，不是 testid）");
  console.log("  · 删除后必须重跑本脚本 —— 会出现级联（删 task → 其消费的 interaction 变死 → 其消费的定位器变死）");
  console.log("  · 本脚本只看静态 import；若存在动态 import() 或字符串拼装路径，结论会失真（本仓库实测无）");
}

if (STRICT && total > 0) process.exit(1);
