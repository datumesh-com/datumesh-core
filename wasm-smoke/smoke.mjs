// Runs the smoke module in Node: node wasm-smoke/smoke.mjs <path to .wasm>
import { readFileSync } from "node:fs";
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), {});
const result = instance.exports.smoke();
console.log(`smoke() = ${result}`);
process.exit(result === 2 ? 0 : 1);
