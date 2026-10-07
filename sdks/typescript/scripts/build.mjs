import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
for (const config of ['tsconfig.json', 'tsconfig.cjs.json']) {
  const result = spawnSync(process.execPath, ['node_modules/typescript/bin/tsc', '-p', config], { stdio: 'inherit' });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
mkdirSync('dist-cjs', { recursive: true });
writeFileSync('dist-cjs/package.json', JSON.stringify({ type: 'commonjs' }) + '\n');
