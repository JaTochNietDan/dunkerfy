// Verify the real archive against the public key shipped in Dunkerfy, without
// needing access to its signing key. XML parsing uses macOS's system libxml2.
import { readFileSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createPublicKey, verify } from 'node:crypto';
import assert from 'node:assert/strict';

const [feed, archive] = process.argv.slice(2);
assert(feed && archive, 'Usage: node scripts/check-update.mjs appcast.xml archive.zip');
const xpath = expression => execFileSync('/usr/bin/xmllint', ['--nonet', '--xpath', expression, feed], {encoding:'utf8'}).trim();
const plist = key => execFileSync('/usr/libexec/PlistBuddy', ['-c', `Print :${key}`, 'Resources/Info.plist'], {encoding:'utf8'}).trim();
assert.equal(xpath('count(/rss/channel/item)'), '1', 'Release feed must contain exactly one current build');
const item = '/rss/channel/item';
const version = plist('CFBundleShortVersionString');
assert.equal(xpath(`string(${item}/*[local-name()='version'])`), plist('CFBundleVersion'));
assert.equal(xpath(`string(${item}/*[local-name()='shortVersionString'])`), version);
assert.equal(xpath(`string(${item}/*[local-name()='minimumSystemVersion'])`), plist('LSMinimumSystemVersion'));
assert.equal(xpath(`string(${item}/enclosure/@url)`), `https://github.com/JaTochNietDan/dunkerfy/releases/download/v${version}/Dunkerfy-${version}-macOS-arm64.zip`);
assert.equal(Number(xpath(`string(${item}/enclosure/@length)`)), statSync(archive).size);
const signature = Buffer.from(xpath(`string(${item}/enclosure/@*[local-name()='edSignature'])`), 'base64');
const publicBytes = Buffer.from(plist('SUPublicEDKey'), 'base64');
assert.equal(publicBytes.length, 32);
assert.equal(signature.length, 64);
const key = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), publicBytes]), format:'der', type:'spki' });
assert(verify(null, readFileSync(archive), key, signature), 'Archive signature is invalid');
console.log('Verified feed version, platform floor, download URL, byte length, and Ed25519 archive signature.');
