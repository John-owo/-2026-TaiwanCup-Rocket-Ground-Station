import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('dependency lockfiles are present and not broadly ignored', () => {
  const gitignore = read('../../../.gitignore');
  assert.equal(gitignore.includes('*lock*'), false);
  assert.equal(existsSync(new URL('../../../src-tauri/Cargo.lock', import.meta.url)), true);
  assert.equal(existsSync(new URL('../../pnpm-lock.yaml', import.meta.url)), true);
});

test('release policy ignores executables but retains verification metadata', () => {
  const gitignore = read('../../../.gitignore');
  assert.match(gitignore, /artifacts\/\*\.exe/u);
  assert.doesNotMatch(gitignore, /artifacts\/\*\.json/u);
});

test('README commands are copyable and document reproducible release builds', () => {
  const readme = read('../../../README.md');
  assert.match(readme, /pnpm install --frozen-lockfile/u);
  assert.match(readme, /\.\\src-ui\/node_modules\/\.bin\/tauri\.CMD/u);
  assert.match(readme, /cargo test --locked/u);
  assert.match(readme, /cargo check --locked/u);
  assert.match(readme, /GitHub Releases/u);
  assert.doesNotMatch(readme, /\.\\src-ui\r?\node_modules/u);
});
