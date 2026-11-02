# Browser checks

From the repository root:

```sh
npm ci --prefix site --ignore-scripts
npm exec --prefix site -- playwright install --with-deps chromium firefox webkit
LUNATUI_SITE_TEST_PORT=4187 npm run test:sandbox --prefix site
```

`LUNATUI_SITE_TEST_PORT` selects the local server port for both the server and
the browser tests. The default is 4173. Use a distinct port when another
worktree has a test server running.

`terminal-alignment.spec.js` checks all six quickstart templates at desktop and
mobile widths in Chromium, Firefox, and WebKit. It verifies that the bundled
full monospace font loads and that glyph positions follow the text cell grid.
WebKit's Range coordinates round to CSS pixels, so the position check permits
one pixel of rounding. Each test attaches a dashboard screenshot.

These engine tests complement manual checks in installed Safari and Chrome.
