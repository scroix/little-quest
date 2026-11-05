import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

// Serves web/dist under /little-quest/ like GitHub Pages, so a root-relative URL in the page 404s here too.
const BASE = '/little-quest/';
const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const PORT = Number(process.env.PORT ?? 4173);
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.css': 'text/css; charset=utf-8',
};

createServer(async (req, res) => {
  const { pathname } = new URL(req.url, 'http://localhost');
  if (pathname === '/') {
    res.writeHead(302, { location: BASE }).end();
    return;
  }
  if (!pathname.startsWith(BASE)) {
    res.writeHead(404).end('not found');
    return;
  }
  const rel = normalize(pathname.slice(BASE.length) || 'index.html').replace(/^(\.\.[/\\])+/, '');
  try {
    const body = await readFile(join(DIST, rel.endsWith('/') ? `${rel}index.html` : rel));
    res.writeHead(200, { 'content-type': TYPES[extname(rel)] ?? 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(PORT, '127.0.0.1', () => console.log(`serving ${DIST} at http://127.0.0.1:${PORT}${BASE}`));
