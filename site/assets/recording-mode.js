// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

(() => {
  const controls = document.querySelector('[data-mode-controls]');
  const radios = [...controls.querySelectorAll('input[name="mode"]')];
  const play = controls.querySelector('[data-play]');
  const preferenceKey = 'lunatui-recording-mode';
  let mode = 'cell';
  try {
    if (localStorage.getItem(preferenceKey) === 'graphite') mode = 'graphite';
  } catch { /* Storage is optional. */ }

  function render() {
    for (const figure of document.querySelectorAll('.mode-recording')) {
      const image = figure.querySelector('img');
      const asset = figure.dataset[mode];
      image.src = 'assets/media/modes/' + asset + (play.checked ? '.webp' : '.png');
      image.alt = figure.dataset[mode + 'Alt'];
      figure.querySelector('figcaption').textContent =
        mode === 'cell' ? 'Cell recording' : figure.dataset.graphiteCaption;
    }
  }

  radios.find(radio => radio.value === mode).checked = true;
  play.checked = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  controls.addEventListener('change', () => {
    mode = radios.find(radio => radio.checked).value;
    try { localStorage.setItem(preferenceKey, mode); } catch { /* Storage is optional. */ }
    render();
  });
  controls.hidden = false;
  render();
})();
