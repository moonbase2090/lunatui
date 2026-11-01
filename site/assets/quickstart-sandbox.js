// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const panel = document.querySelector('[data-sandbox]');

if (panel) {
  const startButton = panel.querySelector('[data-sandbox-start]');
  const fallback = panel.querySelector('[data-sandbox-fallback]');
  const host = panel.querySelector('[data-sandbox-terminal]');
  const status = panel.querySelector('[data-sandbox-status]');
  const templateButtons = [...panel.querySelectorAll('[data-sandbox-template]')];
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  let bootPromise;
  let terminal;
  let fitAddon;
  let Session;
  let session;
  let selectedTemplate = 'minimal';
  let resizeFrame = 0;

  function loadStylesheet(href) {
    return new Promise((resolve, reject) => {
      const existing = document.querySelector(`link[href="${href}"]`);
      if (existing) {
        resolve();
        return;
      }
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = href;
      link.addEventListener('load', resolve, { once: true });
      link.addEventListener('error', reject, { once: true });
      document.head.append(link);
    });
  }

  function loadScript(src) {
    return new Promise((resolve, reject) => {
      const existing = document.querySelector(`script[src="${src}"]`);
      if (existing) {
        if (existing.dataset.loaded === 'true') resolve();
        else existing.addEventListener('load', resolve, { once: true });
        return;
      }
      const script = document.createElement('script');
      script.src = src;
      script.addEventListener('load', () => {
        script.dataset.loaded = 'true';
        resolve();
      }, { once: true });
      script.addEventListener('error', reject, { once: true });
      document.head.append(script);
    });
  }

  function write(bytes) {
    return new Promise((resolve) => {
      if (bytes.length === 0) {
        resolve();
        return;
      }
      terminal.write(decoder.decode(bytes), resolve);
    });
  }

  function recordSize() {
    panel.dataset.cols = String(terminal.cols);
    panel.dataset.rows = String(terminal.rows);
  }

  function openTerminal() {
    terminal = new window.Terminal({
      cols: 48,
      rows: 10,
      cursorBlink: true,
      fontFamily: '"JetBrains Mono", ui-monospace, Menlo, monospace',
      fontSize: 14,
      lineHeight: 1.15,
      screenReaderMode: true,
      scrollback: 0,
      theme: {
        background: '#0a0d1c',
        foreground: '#ece7da',
        cursor: '#b9a7f0',
        selectionBackground: '#39406f',
      },
    });
    fitAddon = new window.FitAddon.FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(host);
    terminal.onData(async (data) => {
      const currentSession = session;
      if (!currentSession) return;
      await write(currentSession.input(encoder.encode(data)));
      terminal.refresh(0, terminal.rows - 1);
      if (session === currentSession && !currentSession.is_running()) {
        status.textContent = 'Session ended. Choose a template to start again.';
      }
    });
  }

  async function fit() {
    const columns = terminal.cols;
    const rows = terminal.rows;
    fitAddon.fit();
    recordSize();
    if (session && (columns !== terminal.cols || rows !== terminal.rows)) {
      terminal.reset();
      await write(session.resize(terminal.cols, terminal.rows));
      terminal.refresh(0, terminal.rows - 1);
    }
  }

  async function selectTemplate(name, focus) {
    selectedTemplate = name;
    templateButtons.forEach((button) => {
      const selected = button.dataset.sandboxTemplate === name;
      button.setAttribute('aria-pressed', String(selected));
    });
    if (!Session) return;

    if (session) session.free();
    session = undefined;
    if (terminal) {
      terminal.dispose();
      host.replaceChildren();
    }
    openTerminal();
    await fit();
    session = new Session(name, terminal.cols, terminal.rows);
    await write(session.tick(performance.now()));
    terminal.refresh(0, terminal.rows - 1);
    status.textContent = `Live ${name} template. Use the keyboard in the terminal.`;
    if (focus) terminal.focus();
  }

  async function boot(focus = false) {
    if (terminal) {
      if (focus) terminal.focus();
      return;
    }
    if (bootPromise) {
      await bootPromise;
      if (focus) terminal?.focus();
      return;
    }

    status.textContent = 'Loading the live Rust demo...';
    startButton.disabled = true;
    bootPromise = (async () => {
      await Promise.all([
        loadStylesheet('assets/sandbox/xterm.css'),
        loadScript('assets/sandbox/xterm.js'),
        import('./sandbox/lunatui_web.js'),
      ]);
      await loadScript('assets/sandbox/addon-fit.js');
      const wasm = await import('./sandbox/lunatui_web.js');
      await wasm.default();
      Session = wasm.Session;

      fallback.hidden = true;
      host.hidden = false;
      startButton.disabled = false;
      startButton.textContent = 'Focus terminal';

      new ResizeObserver(() => {
        cancelAnimationFrame(resizeFrame);
        resizeFrame = requestAnimationFrame(() => fit());
      }).observe(host);

      await selectTemplate(selectedTemplate, focus);
    })();

    try {
      await bootPromise;
    } catch (error) {
      console.error(error);
      bootPromise = undefined;
      startButton.disabled = false;
      status.textContent = 'The live demo could not load. The static frame is still available.';
    }
  }

  startButton.addEventListener('click', () => boot(true));
  templateButtons.forEach((button) => {
    button.addEventListener('click', async () => {
      await boot(false);
      await selectTemplate(button.dataset.sandboxTemplate, true);
    });
  });

  if ('IntersectionObserver' in window) {
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      boot(false);
    }, { rootMargin: '200px' });
    observer.observe(panel);
  }
}
