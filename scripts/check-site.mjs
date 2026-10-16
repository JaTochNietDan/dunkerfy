import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
const root = path.resolve(import.meta.dirname, '../website');
const html = fs.readFileSync(path.join(root, 'index.html'), 'utf8');
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
assert.equal(ids.length, new Set(ids).size, 'HTML IDs must be unique');
for (const [, url] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
  if (url.startsWith('#')) assert(ids.includes(url.slice(1)), `Missing anchor ${url}`);
  else if (url.startsWith('/')) {
    const destination = path.resolve(root, '.' + url);
    assert(destination === root || destination.startsWith(root + path.sep));
    assert(fs.existsSync(destination), `Missing local asset ${url}`);
  } else assert(url.startsWith('https://'), `Unexpected URL ${url}`);
}
for (const required of ['<title>', 'name="description"', 'rel="canonical"', 'not yet verified', 'Developer ID signed and Apple-notarized']) assert(html.includes(required), `Missing ${required}`);
assert.equal((html.match(/<h1\b/g) || []).length, 1);
assert(!html.includes('rules.png'), 'Screenshots are intentionally deferred');
const config = JSON.parse(fs.readFileSync(path.resolve(root, '../wrangler.jsonc'), 'utf8'));
assert.equal(config.assets.directory, './website');
assert(!config.routes, 'Custom domains are managed by oleary-ops Terraform');
execFileSync(process.execPath, ['--check', path.join(root, 'demo.js')], { stdio: 'inherit' });
console.log('Site checks passed: assets, anchors, metadata, release caveats, JS syntax, and hosting ownership.');
