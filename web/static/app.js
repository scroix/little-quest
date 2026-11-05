import init, { WebGame, WebKey } from './little_quest_web.js?v=__VERSION__';

const { Up, Down, Left, Right, Sword, Start } = WebKey;
const KEYBOARD = {
  ArrowUp: Up, KeyW: Up, ArrowDown: Down, KeyS: Down, ArrowLeft: Left, KeyA: Left,
  ArrowRight: Right, KeyD: Right, Space: Sword, KeyZ: Sword, KeyX: Sword,
};
const DEAD_ZONE = 0.12;

const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const stage = $('stage');
const dpad = $('dpad');
const sword = $('sword');
const arms = { [Up]: $('up'), [Down]: $('down'), [Left]: $('left'), [Right]: $('right') };

const params = new URLSearchParams(location.search);
const demo = params.has('demo');
const seed = params.has('seed') ? Number(params.get('seed')) >>> 0 : (Math.random() * 2 ** 32) >>> 0;

/**
 * Largest whole device-pixel scale that fits, and the device-pixel offset that centres the canvas.
 * Layout rounds CSS lengths to 1/64 px, which at most pixel ratios cannot hold a whole number of device
 * pixels. The canvas keeps its 256 by 232 layout box and a transform, which is not rounded, sets the rest.
 */
function fit(availW, availH, dpr, originX, originY, w, h) {
  const scale = Math.max(1, Math.floor(Math.min((availW * dpr) / w, (availH * dpr) / h)));
  return {
    scale,
    x: Math.round((originX + (availW - (w * scale) / dpr) / 2) * dpr),
    y: Math.round((originY + (availH - (h * scale) / dpr) / 2) * dpr),
  };
}

function showFailure(error) {
  console.error(error);
  $('fail').classList.add('show');
  $('app').remove();
}

async function main() {
  if (typeof WebAssembly !== 'object') throw new Error('WebAssembly is not available');
  const wasm = await init({ module_or_path: new URL('./little_quest_web_bg.wasm?v=__VERSION__', import.meta.url) });
  const coarse = matchMedia('(pointer: coarse)').matches;
  const game = new WebGame(seed, demo, coarse);
  const ctx = canvas.getContext('2d', { alpha: false });

  let image = null;
  const blit = () => {
    if (!image || image.data.buffer !== wasm.memory.buffer) {
      image = new ImageData(new Uint8ClampedArray(wasm.memory.buffer, game.pixels_ptr(), game.pixels_len()), game.width(), game.height());
    }
    ctx.putImageData(image, 0, 0);
  };

  let touch = coarse;
  let lifecycle = 'running';
  let last = null;
  let raf = 0;
  let scale = 1;
  let laidOutAt = 0;
  const hist = [0, 0, 0, 0];
  let frames = 0;

  const layout = () => {
    const r = stage.getBoundingClientRect();
    const dpr = devicePixelRatio;
    const f = fit(r.width, r.height, dpr, r.left, r.top, game.width(), game.height());
    canvas.style.transform = `translate(${f.x / dpr}px, ${f.y / dpr}px) scale(${f.scale / dpr})`;
    scale = f.scale;
    laidOutAt = dpr;
  };

  const setTouch = (on) => {
    if (on === touch) return;
    touch = on;
    document.body.classList.toggle('touch', on);
    game.set_touch(on);
    if (lifecycle === 'paused') game.draw_paused();
    blit();
    layout();
  };
  document.body.classList.toggle('touch', touch);

  const held = new Map();
  const down = new Set();
  const sync = () => {
    const want = new Set(held.values());
    for (const k of want) if (!down.has(k)) { down.add(k); game.press(k); }
    for (const k of [...down]) if (!want.has(k)) { down.delete(k); game.release(k); }
    for (const [k, el] of Object.entries(arms)) el.classList.toggle('on', down.has(Number(k)));
    sword.classList.toggle('on', down.has(Sword));
  };
  const releaseAll = () => {
    held.clear();
    sync();
    game.clear_keys();
  };
  const tapStart = () => {
    game.press(Start);
    game.release(Start);
  };

  const loop = (now) => {
    try {
      const before = game.ticks();
      const drew = game.frame(last === null ? 0 : now - last);
      last = now;
      frames++;
      hist[Math.min(3, game.ticks() - before)]++;
      if (drew) blit();
      // A window can move to a display with another pixel ratio without changing size, and no event is reliable for that.
      if (devicePixelRatio !== laidOutAt) layout();
      raf = requestAnimationFrame(loop);
    } catch (error) {
      showFailure(error);
    }
  };
  const run = () => {
    lifecycle = 'running';
    last = null;
    raf = requestAnimationFrame(loop);
  };

  const pause = () => {
    if (lifecycle !== 'running') return;
    cancelAnimationFrame(raf);
    releaseAll();
    if (game.in_play()) {
      lifecycle = 'paused';
      game.draw_paused();
      blit();
    } else {
      lifecycle = 'idle';
    }
  };
  const comeBack = () => { if (lifecycle === 'idle' && !document.hidden) run(); };
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause();
    else comeBack();
  });
  addEventListener('focus', comeBack);
  const away = () => { if (!demo) pause(); };
  addEventListener('blur', away);
  addEventListener('pagehide', away);

  const comeBackOnPress = (e) => {
    if (lifecycle === 'idle') {
      run();
    } else if (lifecycle === 'paused') {
      e.stopImmediatePropagation();
      game.redraw();
      blit();
      run();
    }
  };
  const browserShortcut = (e) => e.metaKey || e.ctrlKey || e.altKey;
  const gameKey = (e) => e.code === 'Enter' || KEYBOARD[e.code] !== undefined;
  addEventListener('keydown', (e) => {
    // Tab would move focus to the browser's toolbar, and the next key press would go there.
    if (gameKey(e) || e.code === 'Tab') e.preventDefault();
    if (!document.hidden && !browserShortcut(e) && !e.repeat) comeBackOnPress(e);
  }, true);
  addEventListener('pointerdown', (e) => {
    setTouch(e.pointerType !== 'mouse');
    if (!document.hidden) comeBackOnPress(e);
  }, true);

  addEventListener('keydown', (e) => {
    if (browserShortcut(e)) {
      // macOS sends no keyup for a key released while Cmd is down.
      releaseAll();
      return;
    }
    if (!gameKey(e)) return;
    setTouch(false);
    if (e.repeat) return;
    if (e.code === 'Enter') tapStart();
    else { held.set(e.code, KEYBOARD[e.code]); sync(); }
  });
  addEventListener('keyup', (e) => {
    if (held.delete(e.code)) sync();
  });

  stage.addEventListener('pointerdown', tapStart);
  const startFromTitle = () => { if (game.mode() === 'title') tapStart(); };

  const source = (e) => `pointer${e.pointerId}`;
  const release = (e) => { if (held.delete(source(e))) sync(); };
  const wireRelease = (el) => {
    for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) el.addEventListener(type, release);
  };

  const dirAt = (e) => {
    const r = dpad.getBoundingClientRect();
    const dx = e.clientX - (r.left + r.width / 2);
    const dy = e.clientY - (r.top + r.height / 2);
    if (Math.hypot(dx, dy) < r.width * DEAD_ZONE) return null;
    if (Math.abs(dx) > Math.abs(dy)) return dx < 0 ? Left : Right;
    return dy < 0 ? Up : Down;
  };
  const steer = (e) => {
    const dir = dirAt(e);
    if (dir === null) held.delete(source(e));
    else held.set(source(e), dir);
    sync();
  };
  dpad.addEventListener('pointerdown', (e) => {
    dpad.setPointerCapture(e.pointerId);
    startFromTitle();
    steer(e);
  });
  dpad.addEventListener('pointermove', (e) => {
    if (dpad.hasPointerCapture(e.pointerId)) steer(e);
  });
  wireRelease(dpad);

  sword.addEventListener('pointerdown', (e) => {
    sword.setPointerCapture(e.pointerId);
    startFromTitle();
    held.set(source(e), Sword);
    sync();
  });
  wireRelease(sword);

  // iOS ignores touch-action for some gestures, so cancel them where they start.
  for (const type of ['touchstart', 'touchmove']) addEventListener(type, (e) => e.preventDefault(), { passive: false });
  addEventListener('wheel', (e) => { if (e.ctrlKey) e.preventDefault(); }, { passive: false });
  for (const type of ['gesturestart', 'gesturechange', 'dblclick', 'contextmenu']) addEventListener(type, (e) => e.preventDefault());

  new ResizeObserver(layout).observe(stage);
  addEventListener('resize', layout);
  layout();
  blit();

  window.__lq = {
    state: () => ({
      mode: game.mode(),
      room: game.room(),
      ticks: game.ticks(),
      modeStartTick: game.mode_start_tick(),
      hero: [game.hero_x(), game.hero_y()],
      halfHearts: game.half_hearts(),
      gems: game.gems(),
      keys: game.keys_held(),
      sword: game.has_sword(),
      lifecycle,
      touch,
      down: [...down].sort(),
      hist: [...hist],
      frames,
      scale,
      dpr: devicePixelRatio,
    }),
  };

  run();
  if (document.hidden) pause();
}

main().catch(showFailure);
