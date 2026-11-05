import { expect, test } from '@playwright/test';
import { Fingers, center, load, state, waitFor } from './helpers.mjs';

const UP = 0, LEFT = 2, RIGHT = 3, SWORD = 4;

const PHONES = {
  'iPhone portrait': { viewport: { width: 390, height: 844 }, deviceScaleFactor: 3 },
  'iPhone landscape': { viewport: { width: 844, height: 390 }, deviceScaleFactor: 3 },
  'small portrait 320x568': { viewport: { width: 320, height: 568 }, deviceScaleFactor: 2 },
  'small landscape 667x375': { viewport: { width: 667, height: 375 }, deviceScaleFactor: 2 },
};

async function phone(browser, spec = PHONES['iPhone portrait']) {
  const context = await browser.newContext({ ...spec, hasTouch: true, isMobile: true });
  const page = await context.newPage();
  return { context, page };
}

async function fingers(context, page) {
  test.skip(context.browser().browserType().name() !== 'chromium', 'multi-touch goes through the Chromium DevTools protocol');
  return new Fingers(await context.newCDPSession(page));
}

async function tapScreen(f, page) {
  const { x, y } = await center(page, '#screen');
  await f.down(1, x, y);
  await f.up(1, x, y);
}

test.describe('touch controls', () => {
  test('a phone gets a D-pad and a sword button, a desktop does not', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page);
    await expect(page.locator('#dpad')).toBeVisible();
    await expect(page.locator('#sword')).toBeVisible();
    await context.close();

    const desktop = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const dp = await desktop.newPage();
    await load(dp);
    await expect(dp.locator('#dpad')).toBeHidden();
    await expect(dp.locator('#sword')).toBeHidden();
    await expect(dp.locator('#source')).toBeVisible();
    await desktop.close();
  });

  for (const [name, spec] of Object.entries(PHONES)) {
    test(`controls and screen do not overlap and arms are at least 44 px: ${name}`, async ({ browser }) => {
      const { context, page } = await phone(browser, spec);
      await load(page);
      const rect = async (sel) => page.locator(sel).boundingBox();
      const boxes = { screen: await rect('#screen'), dpad: await rect('#dpad'), sword: await rect('#sword') };
      const { width, height } = spec.viewport;
      for (const [id, b] of Object.entries(boxes)) {
        expect(b.x, `${id} left`).toBeGreaterThanOrEqual(0);
        expect(b.y, `${id} top`).toBeGreaterThanOrEqual(0);
        expect(b.x + b.width, `${id} right`).toBeLessThanOrEqual(width);
        expect(b.y + b.height, `${id} bottom`).toBeLessThanOrEqual(height);
      }
      const apart = (a, b) => a.x + a.width <= b.x + 0.01 || b.x + b.width <= a.x + 0.01 || a.y + a.height <= b.y + 0.01 || b.y + b.height <= a.y + 0.01;
      expect(apart(boxes.screen, boxes.dpad), 'screen and D-pad').toBe(true);
      expect(apart(boxes.screen, boxes.sword), 'screen and sword').toBe(true);
      expect(apart(boxes.dpad, boxes.sword), 'D-pad and sword').toBe(true);
      for (const id of ['up', 'down', 'left', 'right']) {
        const a = await rect(`#${id}`);
        expect(Math.min(a.width, a.height), `${id} arm`).toBeGreaterThanOrEqual(44);
      }
      expect((await rect('#sword')).width).toBeGreaterThanOrEqual(44);
      await context.close();
    });
  }

  test('tapping the screen starts the game', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page);
    expect((await state(page)).mode).toBe('title');
    const f = await fingers(context, page);
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode !== 'title');
    await context.close();
  });

  test('on the title screen the sword button and the D-pad start the game too', async ({ browser }) => {
    for (const control of ['#sword', '#left']) {
      const { context, page } = await phone(browser);
      await load(page);
      const f = await fingers(context, page);
      const { x, y } = await center(page, control);
      await f.down(1, x, y);
      await waitFor(page, (s) => s.mode !== 'title');
      await f.up(1, x, y);
      await context.close();
    }
  });

  test('the D-pad and the sword button hold together and release independently', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page, '?seed=1');
    const f = await fingers(context, page);
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode === 'playing');
    const right = await center(page, '#right');
    const sword = await center(page, '#sword');
    const before = (await state(page)).hero;

    await f.down(1, right.x, right.y);
    await f.down(2, sword.x, sword.y);
    expect((await state(page)).down).toEqual([RIGHT, SWORD]);
    await expect(page.locator('#sword')).toHaveClass(/\bon\b/);
    await expect(page.locator('#right')).toHaveClass(/\bon\b/);
    await page.waitForTimeout(250);
    expect((await state(page)).hero[0]).toBeGreaterThan(before[0] + 4);

    await f.up(2, sword.x, sword.y);
    expect((await state(page)).down).toEqual([RIGHT]);
    await expect(page.locator('#sword')).not.toHaveClass(/\bon\b/);
    await f.up(1, right.x, right.y);
    expect((await state(page)).down).toEqual([]);
    await context.close();
  });

  test('a thumb can roll between directions without lifting', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page, '?seed=1');
    const f = await fingers(context, page);
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode === 'playing');
    const hub = await center(page, '#dpad');
    const at = async (id) => center(page, `#${id}`);
    const right = await at('right');
    const up = await at('up');
    const left = await at('left');

    await f.down(1, right.x, right.y);
    expect((await state(page)).down).toEqual([RIGHT]);
    await f.move(1, hub.x + (up.x - hub.x) / 2, hub.y + (up.y - hub.y) / 2 + 2);
    await f.move(1, up.x, up.y);
    expect((await state(page)).down).toEqual([UP]);
    await f.move(1, hub.x, hub.y);
    expect((await state(page)).down, 'the dead zone releases').toEqual([]);
    await f.move(1, left.x, left.y);
    expect((await state(page)).down).toEqual([LEFT]);
    await f.move(1, left.x - 400, left.y);
    expect((await state(page)).down, 'dragging off the pad keeps the direction').toEqual([LEFT]);
    await f.up(1, left.x, left.y);
    expect((await state(page)).down).toEqual([]);
    await context.close();
  });

  test('the tap that wakes a paused game does not reach the game', async ({ browser }) => {
    const { context, page } = await phone(browser);
    const f = await fingers(context, page);
    await load(page, '?seed=1');
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode === 'playing');
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'paused');
    const { x, y } = await center(page, '#sword');
    await f.down(1, x, y);
    await waitFor(page, (s) => s.lifecycle === 'running');
    expect((await state(page)).down, 'the sword stays up while the finger is down').toEqual([]);
    await f.up(1, x, y);
    await context.close();
  });

  test('a stylus keeps the touch controls', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page, '?seed=1');
    await page.evaluate(() => document.getElementById('stage').dispatchEvent(new PointerEvent('pointerdown', { pointerType: 'pen', bubbles: true })));
    await expect(page.locator('#dpad')).toBeVisible();
    await context.close();
  });

  test('a cancelled touch releases everything', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page, '?seed=1');
    const f = await fingers(context, page);
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode === 'playing');
    const right = await center(page, '#right');
    const sword = await center(page, '#sword');
    await f.down(1, right.x, right.y);
    await f.down(2, sword.x, sword.y);
    expect((await state(page)).down).toEqual([RIGHT, SWORD]);
    await f.cancel();
    await waitFor(page, (s) => s.down.length === 0);
    await expect(page.locator('#sword')).not.toHaveClass(/\bon\b/);
    await expect(page.locator('#right')).not.toHaveClass(/\bon\b/);
    await context.close();
  });

  test('at game over only the screen restarts', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page, '?seed=10');
    const f = await fingers(context, page);
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode === 'gameover', null, 40_000);
    await page.waitForTimeout(700);
    for (const control of ['#sword', '#left']) {
      const { x, y } = await center(page, control);
      await f.down(1, x, y);
      await page.waitForTimeout(150);
      await f.up(1, x, y);
    }
    expect((await state(page)).mode, 'the controls do not skip the screen').toBe('gameover');
    await tapScreen(f, page);
    await waitFor(page, (s) => s.mode !== 'gameover');
    await context.close();
  });

  test('the page does not scroll, zoom, or hand touches to the browser', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page);
    await page.evaluate(() => {
      window.__prevented = { touchstart: null, touchmove: null };
      for (const type of ['touchstart', 'touchmove']) {
        addEventListener(type, (e) => { window.__prevented[type] = e.defaultPrevented; }, { passive: true });
      }
    });
    const f = await fingers(context, page);
    const a = await center(page, '#screen');
    await f.down(1, a.x, a.y);
    await f.move(1, a.x, a.y - 120);
    await f.up(1, a.x, a.y - 120);
    await f.down(1, a.x, a.y);
    await f.up(1, a.x, a.y);
    await f.down(1, a.x, a.y);
    await f.up(1, a.x, a.y);
    await f.down(1, a.x - 30, a.y);
    await f.down(2, a.x + 30, a.y);
    await f.move(1, a.x - 120, a.y);
    await f.move(2, a.x + 120, a.y);
    await f.up(1, a.x - 120, a.y);
    await f.up(2, a.x + 120, a.y);
    const view = await page.evaluate(() => ({
      scrollY: scrollY,
      scrollX: scrollX,
      scale: visualViewport.scale,
      width: visualViewport.width,
      prevented: window.__prevented,
    }));
    expect(view.scrollX).toBe(0);
    expect(view.scrollY).toBe(0);
    expect(view.scale).toBe(1);
    expect(view.width).toBe(390);
    expect(view.prevented).toEqual({ touchstart: true, touchmove: true });
    await context.close();
  });
});

test.describe('touch on any engine', () => {
  test('a single tap starts the game', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page);
    const { x, y } = await center(page, '#screen');
    await page.touchscreen.tap(x, y);
    await waitFor(page, (s) => s.mode !== 'title');
    expect((await state(page)).touch).toBe(true);
    await context.close();
  });

  test('tapping the sword button on the title screen starts the game', async ({ browser }) => {
    const { context, page } = await phone(browser);
    await load(page);
    const { x, y } = await center(page, '#sword');
    await page.touchscreen.tap(x, y);
    await waitFor(page, (s) => s.mode !== 'title');
    await context.close();
  });
});
