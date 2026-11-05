import { expect, test } from '@playwright/test';
import { center, load, state, waitFor } from './helpers.mjs';

async function hide(page) {
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
  });
}

async function show(page) {
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
    document.dispatchEvent(new Event('visibilitychange'));
  });
}

async function startGame(page) {
  await load(page, '?seed=1');
  await page.keyboard.press('Enter');
  await waitFor(page, (s) => s.mode === 'playing' && s.ticks > 30);
}

const pausedTextPixels = (page) =>
  page.evaluate(() => {
    const { data } = document.getElementById('screen').getContext('2d').getImageData(0, 132, 256, 7);
    let white = 0;
    for (let o = 0; o < data.length; o += 4) if (data[o] === 248 && data[o + 1] === 248 && data[o + 2] === 248) white++;
    return white;
  });

test.describe('pause and resume', () => {
  test('hiding the tab stops the game, releases held keys, and shows PAUSED', async ({ page }) => {
    await startGame(page);
    await page.keyboard.down('ArrowRight');
    await page.waitForTimeout(150);
    expect((await state(page)).down).toEqual([3]);
    const whiteBefore = await pausedTextPixels(page);
    await hide(page);
    await waitFor(page, (s) => s.lifecycle === 'paused');
    expect(await pausedTextPixels(page), 'PAUSED is on screen').toBeGreaterThan(whiteBefore + 20);
    const paused = await state(page);
    expect(paused.down).toEqual([]);
    await page.waitForTimeout(500);
    const later = await state(page);
    expect(later.ticks, 'no ticks while hidden').toBe(paused.ticks);
    expect(later.hero).toEqual(paused.hero);
    await page.keyboard.up('ArrowRight');
  });

  test('coming back shows PAUSED until a key press, and that press does not reach the game', async ({ page }) => {
    await startGame(page);
    await hide(page);
    await waitFor(page, (s) => s.lifecycle === 'paused');
    const before = await state(page);
    await page.waitForTimeout(1500);
    await show(page);
    await page.waitForTimeout(300);
    expect((await state(page)).lifecycle, 'still paused on return').toBe('paused');
    expect((await state(page)).ticks).toBe(before.ticks);

    await page.keyboard.down('ArrowRight');
    await waitFor(page, (s) => s.lifecycle === 'running');
    await page.waitForTimeout(300);
    const after = await state(page);
    expect(after.down, 'the waking key press is swallowed').toEqual([]);
    expect(after.hero).toEqual(before.hero);
    expect(after.hist[3] - before.hist[3], 'no catch-up for the time spent away').toBe(0);
    await page.keyboard.up('ArrowRight');
    await page.keyboard.press('ArrowRight');
  });

  test('a click wakes a paused game', async ({ page }) => {
    await startGame(page);
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'paused');
    const { x, y } = await center(page, '#screen');
    await page.mouse.click(x, y);
    await waitFor(page, (s) => s.lifecycle === 'running');
    expect((await state(page)).mode).toBe('playing');
  });

  test('a key held through the pause does not end it, and browser keys still work', async ({ page }) => {
    await startGame(page);
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'paused');
    const send = (init) =>
      page.evaluate((i) => {
        const e = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...i });
        window.dispatchEvent(e);
        return e.defaultPrevented;
      }, init);
    await send({ code: 'ArrowLeft', repeat: true });
    expect((await state(page)).lifecycle, 'a repeat is not a new press').toBe('paused');
    expect(await send({ code: 'F5' }), 'F5 still reloads').toBe(false);
    expect((await state(page)).lifecycle, 'and it counts as a press').toBe('running');
  });

  test('a press wakes the title screen by itself and also starts the game', async ({ page }) => {
    await load(page, '?seed=1');
    await page.evaluate(() => window.dispatchEvent(new Event('pagehide')));
    await waitFor(page, (s) => s.lifecycle === 'idle');
    await page.keyboard.press('Enter');
    await waitFor(page, (s) => s.mode === 'playing');
    expect((await state(page)).lifecycle).toBe('running');
  });

  test('browser shortcuts pass through a paused game', async ({ page }) => {
    await startGame(page);
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'paused');
    const stopped = await page.evaluate(() => {
      const e = new KeyboardEvent('keydown', { code: 'KeyR', metaKey: true, bubbles: true, cancelable: true });
      window.dispatchEvent(e);
      return e.defaultPrevented;
    });
    expect(stopped, 'Cmd-R still reloads').toBe(false);
    expect((await state(page)).lifecycle, 'and it does not wake the game').toBe('paused');
  });

  test('the title screen has nothing to pause: no PAUSED, it comes back by itself, and one press starts', async ({ page }) => {
    await load(page, '?seed=1');
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'idle');
    const stopped = await state(page);
    await page.waitForTimeout(300);
    expect((await state(page)).ticks, 'the loop stops while away').toBe(stopped.ticks);
    const pausedRow = await page.evaluate(() => Array.from(document.getElementById('screen').getContext('2d').getImageData(0, 135, 256, 1).data));
    expect(pausedRow.filter((v, i) => i % 4 !== 3 && v !== 0), 'no PAUSED text on the title screen').toEqual([]);
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await waitFor(page, (s) => s.lifecycle === 'running' && s.mode === 'title');
    await page.keyboard.press('Enter');
    await waitFor(page, (s) => s.mode === 'playing');
  });

  test('hiding the tab on the title screen comes back by itself', async ({ page }) => {
    await load(page, '?seed=1');
    await hide(page);
    await waitFor(page, (s) => s.lifecycle === 'idle');
    await show(page);
    await waitFor(page, (s) => s.lifecycle === 'running');
  });

  test('losing focus pauses the game', async ({ page }) => {
    await startGame(page);
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitFor(page, (s) => s.lifecycle === 'paused');
  });
});
