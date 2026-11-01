// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const panel = document.querySelector('[data-sandbox]');

if (panel) {
  const startButton = panel.querySelector('[data-sandbox-start]');
  const fallback = panel.querySelector('[data-sandbox-fallback]');
  const host = panel.querySelector('[data-sandbox-terminal]');
  const status = panel.querySelector('[data-sandbox-status]');
  const mobileControls = panel.querySelector('[data-sandbox-mobile]');
  const mobileInput = panel.querySelector('[data-sandbox-input]');
  const templateButtons = [...panel.querySelectorAll('[data-sandbox-template]')];
  const keyButtons = [...panel.querySelectorAll('[data-sandbox-key]')];
  const mobileInputQuery = window.matchMedia('(max-width: 767px), (pointer: coarse)');
  const liveScreenQuery = window.matchMedia('(min-width: 320px)');
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const keyData = {
    ArrowUp: '\u001b[A',
    ArrowDown: '\u001b[B',
    ArrowRight: '\u001b[C',
    ArrowLeft: '\u001b[D',
    Tab: '\t',
    Enter: '\r',
    Escape: '\u001b',
    Backspace: '\u007f',
  };

  let bootPromise;
  let terminal;
  let fitAddon;
  let Session;
  let session;
  let selectedTemplate = 'minimal';
  let resizeFrame = 0;
  let inputQueue = Promise.resolve();
  let pendingInput = '';
  let composing = false;
  let expandedViewportHeight = window.visualViewport?.height ?? window.innerHeight;

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

  function showMobileInput() {
    mobileControls.hidden = false;
  }

  function focusTerminal() {
    if (mobileInputQuery.matches) {
      showMobileInput();
      mobileInput.focus({ preventScroll: true });
    } else {
      terminal?.focus();
    }
  }

  function sendInput(data) {
    if (!data) return inputQueue;
    if (!session) {
      pendingInput += data;
      return inputQueue;
    }

    const currentSession = session;
    inputQueue = inputQueue.then(async () => {
      if (session !== currentSession) return;
      await write(currentSession.input(encoder.encode(data)));
      terminal.refresh(0, terminal.rows - 1);
      if (session === currentSession && !currentSession.is_running()) {
        status.textContent = 'Session ended. Choose a template to start again.';
      }
    }).catch((error) => {
      console.error(error);
      status.textContent = 'The terminal could not process that input. Choose a template to restart it.';
    });
    return inputQueue;
  }

  function configureTerminalInput() {
    const textarea = terminal.textarea;
    if (!textarea) return;
    textarea.autocapitalize = 'none';
    textarea.autocomplete = 'off';
    textarea.setAttribute('autocorrect', 'off');
    textarea.inputMode = 'text';
    textarea.spellcheck = false;
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
    configureTerminalInput();
    terminal.onData((data) => {
      void sendInput(data);
    });
  }

  async function fit() {
    if (!terminal) return;
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

  function scheduleFit() {
    cancelAnimationFrame(resizeFrame);
    resizeFrame = requestAnimationFrame(() => {
      void fit();
    });
  }

  function syncVisualViewport() {
    const viewport = window.visualViewport;
    if (!viewport) return;
    panel.style.setProperty('--sandbox-visual-height', `${Math.round(viewport.height)}px`);
    const inputFocused = document.activeElement === mobileInput;
    if (inputFocused) expandedViewportHeight = Math.max(expandedViewportHeight, viewport.height);
    else expandedViewportHeight = viewport.height;
    const keyboardOpen = mobileInputQuery.matches
      && inputFocused
      && expandedViewportHeight - viewport.height > 120;
    panel.classList.toggle('sandbox-keyboard-open', keyboardOpen);
    scheduleFit();
    if (keyboardOpen && document.activeElement === mobileInput) {
      host.scrollIntoView({ block: 'nearest' });
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
    if (pendingInput) {
      const data = pendingInput;
      pendingInput = '';
      await sendInput(data);
    }
    terminal.refresh(0, terminal.rows - 1);
    status.textContent = `Live ${name} template. Tap the input below or use a keyboard.`;
    if (focus) focusTerminal();
  }

  async function boot(focus = false) {
    if (!liveScreenQuery.matches) {
      status.textContent = 'Static preview. Open this page on a wider screen to run the live terminal.';
      return;
    }
    if (terminal) {
      if (focus) focusTerminal();
      return;
    }
    if (bootPromise) {
      await bootPromise;
      if (focus) focusTerminal();
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
      showMobileInput();

      new ResizeObserver(scheduleFit).observe(host);
      syncVisualViewport();
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

  mobileInput.addEventListener('compositionstart', () => {
    composing = true;
  });
  mobileInput.addEventListener('compositionend', () => {
    composing = false;
    const data = mobileInput.value;
    mobileInput.value = '';
    void sendInput(data);
  });
  mobileInput.addEventListener('input', (event) => {
    if (composing || event.isComposing) return;
    const data = mobileInput.value;
    mobileInput.value = '';
    void sendInput(data);
  });
  mobileInput.addEventListener('keydown', (event) => {
    const data = keyData[event.key];
    if (!data) return;
    event.preventDefault();
    void sendInput(data);
  });
  mobileInput.addEventListener('focus', () => {
    if (!terminal) void boot(false);
  });

  keyButtons.forEach((button) => {
    button.addEventListener('pointerdown', (event) => {
      event.preventDefault();
      void sendInput(keyData[button.dataset.sandboxKey]);
      mobileInput.focus({ preventScroll: true });
    });
    button.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      void sendInput(keyData[button.dataset.sandboxKey]);
      mobileInput.focus({ preventScroll: true });
    });
  });

  host.addEventListener('pointerdown', (event) => {
    if (!mobileInputQuery.matches) return;
    event.preventDefault();
    showMobileInput();
    mobileInput.focus({ preventScroll: true });
  }, { capture: true });

  startButton.addEventListener('click', () => {
    if (mobileInputQuery.matches) {
      showMobileInput();
      mobileInput.focus({ preventScroll: true });
      void boot(false);
    } else {
      void boot(true);
    }
  });
  templateButtons.forEach((button) => {
    button.addEventListener('click', async () => {
      await boot(false);
      await selectTemplate(button.dataset.sandboxTemplate, true);
    });
  });

  window.visualViewport?.addEventListener('resize', syncVisualViewport);
  window.visualViewport?.addEventListener('scroll', syncVisualViewport);

  if (!liveScreenQuery.matches) {
    startButton.hidden = true;
    status.textContent = 'Static preview. Open this page on a wider screen to run the live terminal.';
  } else if ('IntersectionObserver' in window) {
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      void boot(false);
    }, { rootMargin: '200px' });
    observer.observe(panel);
  }
}
