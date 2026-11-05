import { expect, test } from '@playwright/test';
import { PNG } from 'pngjs';
import { H, W, boxBlur, canvasPixels, decode, load, mismatches, state, waitFor } from './helpers.mjs';

const DPRS = [1, 1.25, 1.5, 2, 2.625, 3];
const VIEWPORTS = {
  portrait: { width: 390, height: 844 },
  landscape: { width: 844, height: 390 },
};

async function frozenFrame(browser, viewport, deviceScaleFactor) {
  const context = await browser.newContext({ viewport, deviceScaleFactor, hasTouch: true, isMobile: true });
  const page = await context.newPage();
  await load(page, '?seed=1');
  const box = await page.locator('#screen').boundingBox();
  await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  await waitFor(page, (s) => s.mode === 'playing' && s.ticks > 40 && s.touch);
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await waitFor(page, (s) => s.lifecycle === 'paused');
  return { context, page };
}

function crop(png, x0, y0, w, h) {
  const out = new PNG({ width: w, height: h });
  PNG.bitblt(png, out, x0, y0, w, h, 0, 0);
  return out;
}

async function shot(page) {
  const s = await state(page);
  const at = await page.evaluate(() => {
    const r = document.getElementById('screen').getBoundingClientRect();
    return { left: r.left * devicePixelRatio, top: r.top * devicePixelRatio, width: r.width * devicePixelRatio, height: r.height * devicePixelRatio };
  });
  for (const [k, v] of Object.entries(at)) expect(Math.abs(v - Math.round(v)), `${k} lands on a device pixel`).toBeLessThan(1e-3);
  const full = decode(await page.screenshot());
  const png = crop(full, Math.round(at.left), Math.round(at.top), W * s.scale, H * s.scale);
  expect(Math.round(at.width)).toBe(W * s.scale);
  return { png, scale: s.scale, reference: await canvasPixels(page) };
}

test.describe('crispness', () => {
  for (const [orientation, viewport] of Object.entries(VIEWPORTS)) {
    for (const dpr of DPRS) {
      test(`every game pixel is one uniform block: ${orientation} at DPR ${dpr}`, async ({ browser }) => {
        const { context, page } = await frozenFrame(browser, viewport, dpr);
        const { png, scale, reference } = await shot(page);
        expect(scale, 'the scale is a whole number of device pixels').toBeGreaterThanOrEqual(1);
        expect(png.width).toBe(W * scale);
        expect(png.height).toBe(H * scale);
        expect(mismatches(png, reference, scale)).toBe(0);
        await context.close();
      });
    }
  }

  test('the canvas stays crisp when the pixel ratio changes at the same size', async ({ browser, browserName }) => {
    test.skip(browserName !== 'chromium', 'changing the pixel ratio live goes through the Chromium DevTools protocol');
    const context = await browser.newContext({ viewport: VIEWPORTS.portrait, deviceScaleFactor: 2, hasTouch: true, isMobile: true });
    const page = await context.newPage();
    await load(page, '?seed=1');
    await waitFor(page, (s) => s.scale === 2);
    const client = await context.newCDPSession(page);
    await client.send('Emulation.setDeviceMetricsOverride', { ...VIEWPORTS.portrait, deviceScaleFactor: 3, mobile: true });
    await waitFor(page, (s) => s.dpr === 3 && s.scale === 4);
    const box = await page.evaluate(() => {
      const r = document.getElementById('screen').getBoundingClientRect();
      return [r.left, r.top, r.width, r.height].map((v) => v * devicePixelRatio);
    });
    for (const v of box) expect(Math.abs(v - Math.round(v)), 'the canvas lands on device pixels').toBeLessThan(1e-3);
    expect(Math.round(box[2])).toBe(W * 4);
    await context.close();
  });

  test('a blurred copy of the same frame fails the check', async ({ browser }) => {
    const { context, page } = await frozenFrame(browser, VIEWPORTS.portrait, 3);
    const { png, scale, reference } = await shot(page);
    expect(mismatches(png, reference, scale)).toBe(0);
    const blurred = mismatches(boxBlur(png), reference, scale);
    expect(blurred, 'game pixels with a smeared block').toBeGreaterThan(W * H * 0.05);
    await context.close();
  });

  test('a canvas half a device pixel off the grid fails the check', async ({ browser }) => {
    const { context, page } = await frozenFrame(browser, VIEWPORTS.portrait, 3);
    await page.evaluate(() => {
      const c = document.getElementById('screen');
      c.style.transform = `translate(${0.5 / devicePixelRatio}px, 0) ${c.style.transform}`;
    });
    const s = await state(page);
    const reference = await canvasPixels(page);
    const left = await page.evaluate(() => document.getElementById('screen').getBoundingClientRect().left * devicePixelRatio);
    const top = await page.evaluate(() => document.getElementById('screen').getBoundingClientRect().top * devicePixelRatio);
    const moved = crop(decode(await page.screenshot()), Math.round(left), Math.round(top), W * s.scale, H * s.scale);
    expect(mismatches(moved, reference, s.scale), 'half a device pixel off the grid').toBeGreaterThan(0);
    await context.close();
  });
});
