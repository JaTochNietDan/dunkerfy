// Exercise the real Sparkle tools with disposable test keys, never the release key.
import { mkdtempSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { generateKeyPairSync } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import assert from 'node:assert/strict';

const root = path.resolve(import.meta.dirname, '..');
const temp = mkdtempSync(path.join(tmpdir(), 'dunkerfy-update-test-'));
const run = (exe, args, cwd = temp) => execFileSync(exe, args, {cwd, encoding:'utf8', stdio:['ignore','pipe','pipe']});
try {
  const {publicKey, privateKey} = generateKeyPairSync('ed25519');
  const publicBytes = publicKey.export({format:'der',type:'spki'}).subarray(-32);
  const keyFile = path.join(temp, 'test-key');
  writeFileSync(keyFile, privateKey.export({format:'der',type:'pkcs8'}).subarray(-32).toString('base64'), {mode:0o600});
  const app = path.join(temp, 'Dunkerfy.app');
  run('/usr/bin/ditto', [path.join(root,'dist/Dunkerfy.app'), app]);
  const plist = path.join(app,'Contents/Info.plist');
  run('/usr/bin/plutil', ['-replace','SUPublicEDKey','-string',publicBytes.toString('base64'),plist]);
  run('/usr/bin/codesign', ['--force','--sign','-',app]);
  mkdirSync(path.join(temp,'Resources'));
  copyFileSync(plist,path.join(temp,'Resources/Info.plist'));
  const version = run('/usr/libexec/PlistBuddy',['-c','Print :CFBundleShortVersionString',plist]).trim();
  const releases = path.join(temp,'releases'); mkdirSync(releases);
  const archive = path.join(releases,`Dunkerfy-${version}-macOS-arm64.zip`);
  run('/usr/bin/ditto',['-c','-k','--sequesterRsrc','--keepParent',app,archive]);
  run('generate_appcast',['--ed-key-file',keyFile,'--maximum-deltas','0','--download-url-prefix',`https://github.com/JaTochNietDan/dunkerfy/releases/download/v${version}/`,releases]);
  const feed = path.join(releases,'appcast.xml');
  run('sign_update',['--ed-key-file',keyFile,'--verify',feed]);
  run(process.execPath,[path.join(root,'scripts/check-update.mjs'),feed,archive]);
  const original = readFileSync(archive);
  const corrupted = Buffer.from(original); corrupted[100] ^= 1;
  writeFileSync(archive,corrupted);
  const rejectedArchive = spawnSync(process.execPath,[path.join(root,'scripts/check-update.mjs'),feed,archive],{cwd:temp,encoding:'utf8'});
  assert.notEqual(rejectedArchive.status,0);
  assert(rejectedArchive.stderr.includes('Archive signature is invalid'));
  writeFileSync(archive,original);
  writeFileSync(feed,readFileSync(feed,'utf8').replace('github.com','example.com'));
  const rejectedFeed = spawnSync('sign_update',['--ed-key-file',keyFile,'--verify',feed],{cwd:temp,encoding:'utf8'});
  assert.notEqual(rejectedFeed.status,0, 'Modified signed feed must be rejected');
  console.log('Update security tests passed: generated signed feed/archive accepted; same-size tampered archive and modified feed rejected. Disposable test key removed.');
} finally { rmSync(temp,{recursive:true,force:true}); }
