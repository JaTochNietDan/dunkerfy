'use strict';
// Original, illustrative browser sounds. No audio is fetched or played automatically.
let context;
let activeNodes = [];
let completion;
const notes = { mail: [523.25, 659.25], team: [659.25, 880, 783.99], build: [392, 523.25, 659.25, 783.99] };
const names = { mail: 'Gentle mail chime', team: 'Team mention chime', build: 'Build complete chime' };
const status = document.getElementById('sound-status');
for (const button of document.querySelectorAll('[data-sound]')) {
  button.addEventListener('click', async () => {
    const key = button.dataset.sound;
    try {
      const Audio = window.AudioContext || window.webkitAudioContext;
      if (!Audio) throw new Error('Audio unsupported');
      context ||= new Audio();
      await context.resume();
      for (const node of activeNodes) { try { node.stop(); } catch {} }
      activeNodes = [];
      clearTimeout(completion);
      document.querySelectorAll('.playing').forEach(el => el.classList.remove('playing'));
      button.classList.add('playing');
      notes[key].forEach((frequency, index) => {
        const oscillator = context.createOscillator();
        const gain = context.createGain();
        const at = context.currentTime + index * 0.14;
        oscillator.type = 'sine';
        oscillator.frequency.value = frequency;
        gain.gain.setValueAtTime(0, at);
        gain.gain.linearRampToValueAtTime(0.075, at + 0.012);
        gain.gain.exponentialRampToValueAtTime(0.001, at + 0.4);
        oscillator.connect(gain).connect(context.destination);
        oscillator.start(at);
        oscillator.stop(at + 0.43);
        oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); };
        activeNodes.push(oscillator);
      });
      status.textContent = `Playing ${names[key].toLowerCase()}.`;
      completion = setTimeout(() => { button.classList.remove('playing'); status.textContent = `${names[key]} finished.`; }, notes[key].length * 140 + 450);
    } catch {
      status.textContent = 'Sound preview is unavailable in this browser. You can preview sounds in the Mac app.';
      button.classList.remove('playing');
    }
  });
}
