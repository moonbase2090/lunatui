// Rust highlighting, line numbers, code peek toggle, copy, install tabs.
(() => {
  const KW = /^(as|async|await|break|const|continue|crate|else|enum|fn|for|if|impl|in|let|loop|match|mod|move|mut|pub|ref|return|self|Self|static|struct|super|trait|true|false|type|unsafe|use|where|while)$/;
  const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const TOKEN = /(\/\/[^\n]*)|("(?:\\.|[^"\\])*")|(#!?\[[^\]\n]*\])|\b(\d[\d_]*(?:\.\d+)?(?:_?[a-z]\d+)?)\b|\b([a-z_][a-z0-9_]*!)|\b([A-Za-z_][A-Za-z0-9_]*)\b/g;
  function highlight(src) {
    let out = '', last = 0, m;
    TOKEN.lastIndex = 0;
    while ((m = TOKEN.exec(src))) {
      out += esc(src.slice(last, m.index));
      const [t, c, s, attr, n, mac, id] = m;
      if (c) out += `<span class="tk-c">${esc(c)}</span>`;
      else if (s) out += `<span class="tk-s">${esc(s)}</span>`;
      else if (attr) out += `<span class="tk-m">${esc(attr)}</span>`;
      else if (n) out += `<span class="tk-n">${esc(n)}</span>`;
      else if (mac) out += `<span class="tk-m">${esc(mac)}</span>`;
      else if (KW.test(id)) out += `<span class="tk-k">${esc(id)}</span>`;
      else if (/^[A-Z]/.test(id)) out += `<span class="tk-t">${esc(id)}</span>`;
      else out += esc(t);
      last = m.index + t.length;
    }
    return out + esc(src.slice(last));
  }
  const copyIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/></svg>';
  const okIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>';

  document.querySelectorAll('.code').forEach((box) => {
    const code = box.querySelector('pre code');
    const raw = code.textContent.replace(/\n$/, '');
    const lines = raw.split('\n');
    const lang = code.dataset.lang || 'rust';
    code.innerHTML = lines.map((l) => `<span class="l">${lang === 'rust' ? highlight(l) : esc(l)}${l ? '' : ' '}</span>`).join('');
    const peek = Number(getComputedStyle(box).getPropertyValue('--peek')) || 4;
    if (lines.length <= peek || box.hasAttribute('data-static')) box.classList.add('short');
    // copy
    const copy = document.createElement('button');
    copy.type = 'button'; copy.className = 'copy-btn'; copy.innerHTML = copyIcon;
    copy.setAttribute('aria-label', 'Copy code');
    copy.addEventListener('click', async () => {
      try { await navigator.clipboard.writeText(raw); } catch { /* clipboard blocked */ }
      copy.innerHTML = okIcon; copy.setAttribute('aria-label', 'Copied');
      setTimeout(() => { copy.innerHTML = copyIcon; copy.setAttribute('aria-label', 'Copy code'); }, 1600);
    });
    box.prepend(copy);
    // expand
    if (!box.classList.contains('short')) {
      const more = document.createElement('div');
      more.className = 'more';
      const btn = document.createElement('button');
      btn.type = 'button'; btn.className = 'btn-ghost';
      const label = () => box.classList.contains('open') ? 'Hide code' : `View all ${lines.length} lines`;
      btn.textContent = label();
      btn.setAttribute('aria-expanded', 'false');
      btn.addEventListener('click', () => {
        box.classList.toggle('open');
        btn.setAttribute('aria-expanded', String(box.classList.contains('open')));
        btn.textContent = label();
      });
      more.append(btn);
      box.append(more);
    }
  });

  // install tabs
  document.querySelectorAll('[data-tabs]').forEach((root) => {
    const tabs = [...root.querySelectorAll('[role=tab]')];
    const select = (t) => tabs.forEach((x) => {
      const on = x === t;
      x.setAttribute('aria-selected', String(on)); x.tabIndex = on ? 0 : -1;
      document.getElementById(x.getAttribute('aria-controls')).hidden = !on;
    });
    tabs.forEach((t, i) => {
      t.addEventListener('click', () => select(t));
      t.addEventListener('keydown', (e) => {
        const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
        if (d) { const n = tabs[(i + d + tabs.length) % tabs.length]; select(n); n.focus(); }
      });
    });
  });
})();
