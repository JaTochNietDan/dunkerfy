// Original Dunkerfy sound design. Reproducible 44.1 kHz mono PCM, no dependencies.
// MIT licensed alongside the app. These are original effects, not samples from games/films.
import fs from 'node:fs';
import path from 'node:path';
const out=path.resolve(import.meta.dirname,'../Resources/Sounds'); fs.mkdirSync(out,{recursive:true});
const sr=44100, tau=2*Math.PI;
let seed=28491;
const noise=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/2147483648-1;};
const sin=(f,t)=>Math.sin(tau*f*t);
const saw=(f,t)=>2*((f*t)%1)-1;
const tri=(f,t)=>2*Math.abs(saw(f,t))-1;
const decay=(t,k=8)=>Math.exp(-t*k);
const sweep=(a,b,t,d)=>Math.sin(tau*(a*t+(b-a)*t*t/(2*d)));
const note=(f,t,d=.25)=>t<0||t>d?0:sin(f,t)*Math.min(t/.008,1)*Math.pow(1-t/d,1.8);
const seq=(notes,t,step=.14,d=.22)=>notes.reduce((s,f,i)=>s+note(f,t-i*step,d),0);
const bank=[
 ['sad-trombone','Sad trombone','Fun','A gloriously unhelpful wah-wah.',1.65,t=>{const i=Math.floor(t/.38),f=[294,277,262,220][Math.min(i,3)],u=t% .38;return .5*(sin(f-18*u,u)+.3*sin(f*2,u)+.15*saw(f,u))*Math.sin(Math.PI*Math.min(u/.37,1))*(1+.25*sin(7,t));}],
 ['air-horn','Air horn','Fun','Three tiny victory blasts.',1.15,t=>{let u=t% .36;return u<.25?(sin(440,u)+sin(554,u)+sin(660,u))*.25*Math.min(u/.015,1)*Math.min((.25-u)/.025,1):0;}],
 ['rimshot','Ba dum tss','Fun','For the message that thinks it is funny.',1.2,t=>note(120,t,.16)+note(140,t-.18,.16)+(t>.36?noise()*decay(t-.36,7)*.42:0)],
 ['record-scratch','Record scratch','Fun','Well, that got awkward.',.75,t=>noise()*.35*(.5+.5*sin(38-24*t,t))*Math.sin(Math.PI*t/.75)+sweep(900,90,t,.75)*.3*decay(t,2)],
 ['boing','Rubber boing','Fun','Spring-loaded nonsense.',.95,t=>sin(180+95*Math.exp(-t*5)*Math.sin(t*38),t)*decay(t,4)],
 ['slide-whistle','Slide whistle','Fun','An extremely unserious entrance.',1.0,t=>sweep(420,1900,t,1)*Math.sin(Math.PI*t)*(.7+.12*sin(8,t))],
 ['rubber-duck','Rubber duck','Fun','Two little squeaks.',.8,t=>{let u=t% .38;return u<.25?sweep(700,430,u,.25)*(1+.5*sin(93,u))*Math.sin(Math.PI*u/.25):0;}],
 ['whoopee','Whoopee','Fun','Peak sophistication.',.7,t=>saw(65+30*sin(5,t),t)*(.65+.35*noise())*Math.sin(Math.PI*t/.7)],
 ['tiny-violin','Tiny violin','Fun','A very small amount of sympathy.',1.5,t=>seq([659,622,587,523],t,.33,.36)*.5+.18*saw(660+6*sin(6,t),t)*Math.sin(Math.PI*t/1.5)],
 ['crickets','Crickets','Fun','For a conversation going nowhere.',1.4,t=>sin(3900,t)*Math.pow(Math.max(0,sin(28,t)),4)*(Math.sin(tau*2*t)>0?1:.08)*.5],
 ['drama','Dramatic sting','Fun','A miniature plot twist.',1.3,t=>(sin(130.81,t)+sin(138.59,t)+sin(196,t))*.22*decay(t,3)+noise()*.18*decay(t,18)],
 ['bonk','Bonk','Fun','A soft cartoon collision.',.35,t=>sweep(420,90,t,.35)*decay(t,12)+noise()*.2*decay(t,40)],
 ['ufo','UFO incoming','Fun','The group chat has visitors.',1.2,t=>sin(650+300*sin(3,t),t)*Math.sin(Math.PI*t/1.2)*.6],
 ['brain-buffer','Brain buffering','Fun','Please hold while thoughts load.',1.4,t=>seq([330,330,330,262,220,165],t,.19,.12)],
 ['coin','Pocket coin','Arcade','A bright little reward.',.5,t=>seq([988,1976],t,.09,.32)],
 ['level-up','Level up','Arcade','A tiny victory fanfare.',.95,t=>seq([523,659,784,1047],t,.13,.42)],
 ['power-up','Power up','Arcade','Charge it. Launch it.',.8,t=>sweep(180,1450,t,.8)*(.6+.4*Math.sin(tau*18*t))*Math.sin(Math.PI*t/.8)],
 ['game-over','Game over','Arcade','A descending digital sigh.',1.15,t=>seq([523,440,349,262],t,.22,.34)],
 ['laser','Pew pew','Arcade','Two shots across the inbox.',.65,t=>{let u=t% .3;return sweep(1900,120,u,.3)*decay(u,16);}],
 ['checkpoint','Checkpoint','Arcade','Progress, safely tucked away.',.7,t=>seq([660,880,1320],t,.11,.33)],
 ['combo','Combo breaker','Arcade','Quick, crunchy, satisfying.',.7,t=>seq([330,494,740,1110,1480],t,.07,.18)],
 ['pixel-hop','Pixel hop','Arcade','A cheerful little jump.',.28,t=>sweep(230,820,t,.28)*Math.sin(Math.PI*t/.28)],
 ['bubble','Bubble','Calm','One soft, rounded pop.',.4,t=>sweep(1100,420,t,.4)*decay(t,16)],
 ['glass-drop','Glass drop','Calm','A clear note with a gentle tail.',.9,t=>(sin(1400,t)+.3*sin(3732,t))*.65*decay(t,7)],
 ['soft-orbit','Soft orbit','Calm','A warm, floating two-note cue.',1.25,t=>seq([440,659],t,.22,.95)],
 ['bamboo','Bamboo','Calm','Dry, woody, understated.',.5,t=>sin(620,t)*decay(t,16)+sin(1480,t)*.25*decay(t,30)],
 ['doorbell','Little doorbell','Calm','A friendly ding-dong.',1.25,t=>seq([880,698],t,.35,.85)],
 ['stardust','Stardust','Calm','Three light, sparkling notes.',1.2,t=>seq([1318,1568,2093],t,.16,.65)],
 ['warm-pulse','Warm pulse','Calm','A low-key nudge.',.65,t=>seq([220,330],t,.17,.4)],
];
function wav(id,values){
 let peak=Math.max(.001,...values.filter((_,i)=>i%8===0).map(Math.abs));
 // Full scan keeps narrow noise transients below the peak ceiling too.
 for(const x of values)peak=Math.max(peak,Math.abs(x));
 const bytes=Buffer.alloc(44+values.length*2);bytes.write('RIFF');bytes.writeUInt32LE(bytes.length-8,4);bytes.write('WAVEfmt ',8);bytes.writeUInt32LE(16,16);bytes.writeUInt16LE(1,20);bytes.writeUInt16LE(1,22);bytes.writeUInt32LE(sr,24);bytes.writeUInt32LE(sr*2,28);bytes.writeUInt16LE(2,32);bytes.writeUInt16LE(16,34);bytes.write('data',36);bytes.writeUInt32LE(values.length*2,40);
 values.forEach((x,i)=>{let fade=Math.min(1,i/220,(values.length-1-i)/440);bytes.writeInt16LE(Math.round(Math.max(-1,Math.min(1,x/peak*.7*fade))*32767),44+i*2);});fs.writeFileSync(path.join(out,id+'.wav'),bytes);
}
const catalog=[];
for(const [id,name,category,description,duration,fn] of bank){seed=28491;wav(id,Array.from({length:Math.round(sr*duration)},(_,i)=>fn(i/sr)));catalog.push({id,name,category,description,duration});}
const source=process.argv[2];
if(source){const b=fs.readFileSync(source);let offset=12,data;while(offset+8<=b.length){let size=b.readUInt32LE(offset+4);if(b.toString('ascii',offset,offset+4)==='data'){data=b.subarray(offset+8,offset+8+size);break;}offset+=8+size+(size%2);}if(!data)throw Error('Missing PCM data');const start=Math.round(27*sr),count=Math.round(1.65*sr);wav('wilhelm',Array.from({length:count},(_,i)=>data.readInt16LE((start+i)*2)/32768));}
catalog.unshift({id:'wilhelm',name:'Wilhelm scream',category:'Fun',description:'The actual classic Hollywood scream.',duration:1.65});
fs.writeFileSync(path.join(out,'catalog.json'),JSON.stringify(catalog,null,2)+'\n');
console.log(`Generated ${bank.length} original effects; catalog contains ${catalog.length} presets.`);
