import path from "node:path";
import { fileURLToPath } from "node:url";
export function designerPaths(repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../../..")) {
  const root = path.resolve(repoRoot);
  return Object.freeze({ root, state: path.join(root, ".cutright-tools", "designer"), config: path.join(root, "config", "designer.json") });
}
export default designerPaths;
