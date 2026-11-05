import { expect, test } from '@playwright/test';
import { load, waitFor } from './helpers.mjs';

// Trace recording screencasts the page, which on WebKit disturbs the frame timing this measures.
test.use({ trace: 'off' });

async function startGame(page) {
  await load(page, '?seed=1');
  await page.keyboard.press('Enter');
  await waitFor(page, (s) => s.mode === 'playing' && s.ticks > 30);
}

const fakeDisplay = (hz) => `(() => {
  const t0 = performance.now();
  let queue = new Map();
  let handle = 0;
  let last = 0;
  let armed = false;
  const pump = () => {
    armed = false;
    const frame = Math.round(((performance.now() - t0) * ${hz}) / 1000);
    if (frame <= last) return arm();
    last = frame;
    const due = queue;
    queue = new Map();
    for (const cb of due.values()) cb(Math.round(t0 + (frame * 1000) / ${hz}));
  };
  const arm = () => {
    if (armed) return;
    armed = true;
    setTimeout(pump, Math.max(1, t0 + ((last + 1) * 1000) / ${hz} - performance.now()));
  };
  window.requestAnimationFrame = (cb) => {
    queue.set(++handle, cb);
    arm();
    return handle;
  };
  window.cancelAnimationFrame = (h) => queue.delete(h);
})()`;

const sample = (page) => page.evaluate(() => ({ s: window.__lq.state(), now: performance.now() }));

async function measure(page, ms) {
  await page.waitForFunction(() => window.__lq);
  const a = await sample(page);
  await page.waitForTimeout(ms);
  const b = await sample(page);
  const seconds = (b.now - a.now) / 1000;
  return {
    hist: b.s.hist.map((n, i) => n - a.s.hist[i]),
    frames: b.s.frames - a.s.frames,
    ticks: b.s.ticks - a.s.ticks,
    perSecond: (b.s.ticks - a.s.ticks) / seconds,
  };
}

test.describe('ticks per frame', () => {
  for (const hz of [100, 144]) {
    test(`a ${hz} Hz display runs in real time`, async ({ page }) => {
      test.setTimeout(120_000);
      await page.addInitScript(fakeDisplay(hz));
      await startGame(page);
      const m = await measure(page, 4000);
      test.info().annotations.push({ type: 'ticks per frame', description: `${hz} Hz: 0:${m.hist[0]} 1:${m.hist[1]} 2:${m.hist[2]} 3+:${m.hist[3]}, ${m.frames} frames, ${m.ticks} ticks` });
      expect(m.perSecond, 'ticks per second').toBeGreaterThan(57);
      expect(m.perSecond, 'ticks per second').toBeLessThan(63);
    });
  }

  for (const hz of [30, 120]) {
    test(`a ${hz} Hz display never makes a frame with the wrong number of ticks`, async ({ page }) => {
      test.setTimeout(120_000);
      await page.addInitScript(fakeDisplay(hz));
      await startGame(page);
      const m = await measure(page, 4000);
      test.info().annotations.push({ type: 'ticks per frame', description: `${hz} Hz: 0:${m.hist[0]} 1:${m.hist[1]} 2:${m.hist[2]} 3+:${m.hist[3]}, ${m.frames} frames, ${m.ticks} ticks` });
      expect(m.perSecond, 'ticks per second').toBeGreaterThan(54);
      expect(m.perSecond, 'ticks per second').toBeLessThan(66);
      const odd = hz === 30 ? m.hist[0] + m.hist[1] + m.hist[3] : m.hist[2] + m.hist[3];
      expect(odd / m.frames).toBeLessThan(0.02);
    });
  }

  test('the real display makes one tick per frame', async ({ page }) => {
    await startGame(page);
    const m = await measure(page, 4000);
    test.info().annotations.push({ type: 'ticks per frame', description: `real: 0:${m.hist[0]} 1:${m.hist[1]} 2:${m.hist[2]} 3+:${m.hist[3]}, ${m.frames} frames, ${m.ticks} ticks` });
    expect(m.perSecond).toBeGreaterThan(54);
    expect(m.perSecond).toBeLessThan(66);
    expect((m.hist[0] + m.hist[2] + m.hist[3]) / m.frames).toBeLessThan(0.1);
  });

  for (const rate of [4, 6]) {
    test(`under ${rate}x CPU throttling the game keeps its pace`, async ({ page, browserName }) => {
      test.skip(browserName !== 'chromium', 'CPU throttling is a Chromium DevTools feature');
      await startGame(page);
      const client = await page.context().newCDPSession(page);
      await client.send('Emulation.setCPUThrottlingRate', { rate });
      const m = await measure(page, 4000);
      await client.send('Emulation.setCPUThrottlingRate', { rate: 1 });
      test.info().annotations.push({ type: 'ticks per frame', description: `${rate}x throttle: 0:${m.hist[0]} 1:${m.hist[1]} 2:${m.hist[2]} 3+:${m.hist[3]}, ${m.frames} frames, ${m.ticks} ticks` });
      expect(m.perSecond, 'ticks per second').toBeGreaterThan(54);
      expect(m.perSecond, 'ticks per second').toBeLessThan(66);
      expect((m.hist[0] + m.hist[2] + m.hist[3]) / m.frames).toBeLessThan(0.05);
    });
  }
});
