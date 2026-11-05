import { expect, test } from '@playwright/test';
import { load, state, waitFor } from './helpers.mjs';

const playing = (s) => s.mode === 'playing';

async function startGame(page, query = '?seed=1') {
  await load(page, query);
  await page.keyboard.press('Enter');
  await waitFor(page, playing);
  return state(page);
}

test.describe('keyboard', () => {
  test('the page loads with no console errors, no failed requests, and versioned assets', async ({ page }) => {
    const problems = [];
    const urls = [];
    page.on('console', (m) => ['error', 'warning'].includes(m.type()) && problems.push(`${m.type()}: ${m.text()}`));
    page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`));
    page.on('requestfailed', (r) => problems.push(`failed: ${r.url()}`));
    page.on('response', (r) => {
      urls.push(r.url());
      if (r.status() >= 400) problems.push(`${r.status()}: ${r.url()}`);
    });
    const wasm = page.waitForResponse((r) => r.url().includes('_bg.wasm'));
    await load(page);
    expect((await wasm).headers()['content-type']).toBe('application/wasm');
    expect(problems).toEqual([]);
    const stamped = urls.filter((u) => /little_quest_web(_bg)?\.(js|wasm)|app\.js/.test(u));
    expect(stamped.length).toBeGreaterThanOrEqual(3);
    for (const u of stamped) expect(u, 'asset URL carries the build hash').toMatch(/\?v=[0-9a-f]{10}$/);
  });

  test('Enter starts the game and the sword keys do not', async ({ page }) => {
    await load(page, '?seed=1');
    await page.keyboard.press('Space');
    await page.keyboard.press('KeyZ');
    await page.keyboard.press('KeyX');
    await page.waitForTimeout(200);
    expect((await state(page)).mode).toBe('title');
    await page.keyboard.press('Enter');
    await waitFor(page, playing);
  });

  for (const [name, keys] of [
    ['arrows', { up: 'ArrowUp', down: 'ArrowDown', left: 'ArrowLeft', right: 'ArrowRight' }],
    ['WASD', { up: 'KeyW', down: 'KeyS', left: 'KeyA', right: 'KeyD' }],
  ]) {
    test(`${name} walk the hero in each direction and stop on release`, async ({ page }) => {
      const start = await startGame(page);
      const axis = { up: [1, -1], down: [1, 1], left: [0, -1], right: [0, 1] };
      for (const dir of ['right', 'down', 'left', 'up']) {
        const before = (await state(page)).hero;
        await page.keyboard.down(keys[dir]);
        await page.waitForTimeout(250);
        await page.keyboard.up(keys[dir]);
        const after = (await state(page)).hero;
        const [i, sign] = axis[dir];
        expect((after[i] - before[i]) * sign, `${dir} moved`).toBeGreaterThan(4);
        await page.waitForTimeout(100);
        expect((await state(page)).hero, `${dir} stopped`).toEqual(after);
      }
      expect((await state(page)).mode).toBe('playing');
      expect(start.hero).toBeTruthy();
    });
  }

  test('the most recently pressed direction wins and releasing it hands back the older one', async ({ page }) => {
    await startGame(page);
    const a = (await state(page)).hero;
    await page.keyboard.down('ArrowRight');
    await page.waitForTimeout(150);
    await page.keyboard.down('ArrowDown');
    expect((await state(page)).down).toEqual([1, 3]);
    const b = (await state(page)).hero;
    await page.waitForTimeout(250);
    const c = (await state(page)).hero;
    expect(c[1]).toBeGreaterThan(b[1] + 4);
    expect(c[0]).toBe(b[0]);
    await page.keyboard.up('ArrowDown');
    await page.waitForTimeout(150);
    expect((await state(page)).hero[0]).toBeGreaterThan(c[0] + 3);
    await page.keyboard.up('ArrowRight');
    expect(b[0]).toBeGreaterThan(a[0]);
  });

  test('a trackpad pinch does not zoom the page', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'pinch synthesis goes through the Chromium DevTools protocol');
    await load(page, '?seed=1');
    const client = await page.context().newCDPSession(page);
    await client.send('Input.synthesizePinchGesture', { x: 400, y: 300, scaleFactor: 2, gestureSourceType: 'mouse' });
    expect(await page.evaluate(() => visualViewport.scale)).toBe(1);
  });

  test('a stray Tab leaves the game keys working', async ({ page }) => {
    await startGame(page);
    for (let i = 0; i < 2; i++) await page.keyboard.press('Tab');
    await page.keyboard.down('ArrowLeft');
    const s = await state(page);
    expect(s.lifecycle, 'focus stayed on the page').toBe('running');
    expect(s.down).toEqual([2]);
    await page.keyboard.up('ArrowLeft');
  });

  test('pressing Cmd lets go of held keys, because macOS drops their keyup', async ({ page }) => {
    await startGame(page);
    await page.keyboard.down('ArrowLeft');
    await page.keyboard.down('Meta');
    expect((await state(page)).down).toEqual([]);
    await page.keyboard.up('Meta');
    await page.keyboard.up('ArrowLeft');
  });

  test('holding a direction key keeps one press down', async ({ page }) => {
    await startGame(page);
    await page.keyboard.down('ArrowRight');
    for (let i = 0; i < 5; i++) await page.keyboard.down('ArrowRight');
    expect((await state(page)).down).toEqual([3]);
    await page.keyboard.up('ArrowRight');
    expect((await state(page)).down).toEqual([]);
  });

  test('game over waits for Enter, and Enter leaves it', async ({ page }) => {
    await startGame(page, '?seed=10');
    await page.keyboard.down('Enter');
    await waitFor(page, (s) => s.mode === 'gameover', null, 40_000);
    await page.keyboard.press('Space');
    await page.waitForTimeout(700);
    expect((await state(page)).mode, 'the sword key does not skip the screen').toBe('gameover');
    await page.keyboard.down('Enter');
    await page.waitForTimeout(100);
    expect((await state(page)).mode, 'a held Enter repeating does not skip the screen').toBe('gameover');
    await page.keyboard.up('Enter');
    await page.keyboard.press('Enter');
    await waitFor(page, (s) => s.mode !== 'gameover');
  });

  for (const [name, pattern] of [['wasm', '**/*_bg.wasm*'], ['JavaScript', '**/little_quest_web.js*']]) {
    test(`a failed ${name} download shows a message instead of a black screen`, async ({ page }) => {
      await page.route(pattern, (route) => route.abort());
      await page.goto('./');
      await expect(page.locator('#fail')).toBeVisible();
      await expect(page.locator('#fail')).toContainText('hit a problem');
    });
  }

  test('the bot wins seed 1 on tick 2574', async ({ page }) => {
    test.setTimeout(120_000);
    await load(page, '?demo&seed=1');
    await waitFor(page, (s) => s.mode === 'victory', null, 100_000);
    expect((await state(page)).modeStartTick, 'the tick the win began on').toBe(2574);
  });
});
