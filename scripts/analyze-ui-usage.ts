import { promises as fs } from "fs";
import path from "path";

const root = path.resolve(process.cwd(), "web");
const uiUsage = new Map<string, number>();
const fileMatches = new Map<string, Set<string>>();

async function walk(dir: string) {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            await walk(fullPath);
            continue;
        }
        if (!/\.(ts|tsx)$/.test(entry.name)) continue;

        const content = await fs.readFile(fullPath, "utf8");
        const regex = /@\/components\/ui\/([\w-]+)/g;
        let match: RegExpExecArray | null;
        while ((match = regex.exec(content))) {
            const key = match[1];
            uiUsage.set(key, (uiUsage.get(key) ?? 0) + 1);
            if (!fileMatches.has(key)) {
                fileMatches.set(key, new Set());
            }
            fileMatches.get(key)?.add(path.relative(root, fullPath));
        }
    }
}

(async () => {
    await walk(root);
    const sorted = [...uiUsage.entries()].sort((a, b) =>
        a[0].localeCompare(b[0])
    );
    for (const [component, count] of sorted) {
        const files = [...(fileMatches.get(component) ?? [])]
            .map((f) => `  - ${f}`)
            .join("\n");
        console.log(`${component}: ${count}`);
        if (files) {
            console.log(files);
        }
    }
})();
