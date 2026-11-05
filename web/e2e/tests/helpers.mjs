import { PNG } from 'pngjs';

export const W = 256;
export const H = 232;

export async function load(page, query = '') {
  await page.goto(`./${query}`);
  await page.waitForFunction(() => window.__lq);
}

export const state = (page) => page.evaluate(() => window.__lq.state());

export const waitFor = (page, predicate, arg, timeout) =>
  page.waitForFunction(`(${predicate})(window.__lq.state(), ${JSON.stringify(arg ?? null)})`, null, { timeout });

export async function center(page, selector) {
  const box = await page.locator(selector).boundingBox();
  return { x: box.x + box.width / 2, y: box.y + box.height / 2, box };
}

export const canvasPixels = (page) =>
  page.evaluate(() => {
    const c = document.getElementById('screen');
    return Array.from(c.getContext('2d').getImageData(0, 0, c.width, c.height).data);
  });

export function decode(buffer) {
  return PNG.sync.read(buffer);
}

export function mismatches(shot, reference, scale) {
  if (shot.width !== W * scale || shot.height !== H * scale) {
    throw new Error(`screenshot is ${shot.width}x${shot.height}, expected ${W * scale}x${H * scale}`);
  }
  let bad = 0;
  for (let gy = 0; gy < H; gy++) {
    for (let gx = 0; gx < W; gx++) {
      const r = (gy * W + gx) * 4;
      let ok = true;
      for (let dy = 0; dy < scale && ok; dy++) {
        for (let dx = 0; dx < scale; dx++) {
          const s = ((gy * scale + dy) * shot.width + gx * scale + dx) * 4;
          if (shot.data[s] !== reference[r] || shot.data[s + 1] !== reference[r + 1] || shot.data[s + 2] !== reference[r + 2]) {
            ok = false;
            break;
          }
        }
      }
      if (!ok) bad++;
    }
  }
  return bad;
}

export function boxBlur(png) {
  const out = new PNG({ width: png.width, height: png.height });
  for (let y = 0; y < png.height; y++) {
    for (let x = 0; x < png.width; x++) {
      const sum = [0, 0, 0];
      let n = 0;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const xx = x + dx;
          const yy = y + dy;
          if (xx < 0 || yy < 0 || xx >= png.width || yy >= png.height) continue;
          const o = (yy * png.width + xx) * 4;
          for (let c = 0; c < 3; c++) sum[c] += png.data[o + c];
          n++;
        }
      }
      const o = (y * png.width + x) * 4;
      for (let c = 0; c < 3; c++) out.data[o + c] = Math.round(sum[c] / n);
      out.data[o + 3] = 255;
    }
  }
  return out;
}

/** CDP touch input, one event per finger. Chromium only. */
export class Fingers {
  constructor(client) {
    this.client = client;
  }

  send(type, id, x, y) {
    return this.client.send('Input.dispatchTouchEvent', { type, touchPoints: [{ id, x, y }] });
  }

  down(id, x, y) {
    return this.send('touchStart', id, x, y);
  }

  move(id, x, y) {
    return this.send('touchMove', id, x, y);
  }

  up(id, x, y) {
    return this.send('touchEnd', id, x, y);
  }

  cancel() {
    return this.client.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
  }
}
