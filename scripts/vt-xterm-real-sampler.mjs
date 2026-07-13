// 真实页面 xterm 终端区域 VT 采样器。
//
// 验证 xterm 终端输出区域(运行后挂载)是否随 VT 圆形展开动画变色。
// 与编辑器采样器不同:xterm 在用户点「运行」后才挂载,且背景是 inline style。
//
// 运行:node scripts/vt-xterm-real-sampler.mjs
// 前提:make dev 已启动。

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { inflateSync } from "node:zlib";

const __require = createRequire(import.meta.url);

function loadChromium() {
  const candidates = [];
  const npxCache = join(homedir(), ".npm", "_npx");
  if (existsSync(npxCache)) {
    for (const hash of readdirSync(npxCache)) {
      candidates.push(join(npxCache, hash, "node_modules", "playwright-core"));
    }
  }
  for (const dir of candidates) {
    if (!existsSync(dir)) continue;
    try {
      const pw = __require(dir);
      if (pw?.chromium?.launch) return pw.chromium;
    } catch {}
  }
  throw new Error("playwright not found");
}

function decodePng(buf) {
  let off = 8, w = 0, h = 0, ct = 0;
  const idat = [];
  while (off < buf.length) {
    const len = buf.readUInt32BE(off); off += 4;
    const type = buf.toString("ascii", off, off + 4); off += 4;
    const data = buf.subarray(off, off + len); off += len + 4;
    if (type === "IHDR") { w = data.readUInt32BE(0); h = data.readUInt32BE(4); ct = data[9]; }
    else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
  }
  const inf = inflateSync(Buffer.concat(idat));
  const ch = ct === 6 ? 4 : 3;
  const bpp = ch, stride = w * bpp;
  let prev = Buffer.alloc(stride), io = 0;
  const raw = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = inf[io++], line = inf.subarray(io, io + stride); io += stride;
    const out = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const c = line[x], l = x >= bpp ? out[x - bpp] : 0, u = prev[x], ul = x >= bpp ? prev[x - bpp] : 0;
      let v;
      switch (f) {
        case 0: v = c; break;
        case 1: v = (c + l) & 0xff; break;
        case 2: v = (c + u) & 0xff; break;
        case 3: v = (c + ((l + u) >> 1)) & 0xff; break;
        case 4: { const pp = l + u - ul, pa = Math.abs(pp-l), pb = Math.abs(pp-u), pc = Math.abs(pp-ul); v = (c + (pa<=pb&&pa<=pc?l:pb<=pc?u:ul)) & 0xff; } break;
        default: throw new Error("f" + f);
      }
      out[x] = v;
    }
    out.copy(raw, y * stride); prev = out;
  }
  return { r: raw[0], g: raw[1], b: raw[2] };
}
const hex = ({ r, g, b }) => "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");

async function main() {
  const chromium = loadChromium();
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  await page.emulateMedia({ reducedMotion: "no-preference" });

  console.log("[info] navigating to /post/rust");
  await page.goto("http://localhost:8080/post/rust", { waitUntil: "networkidle", timeout: 30000 });
  await page.waitForTimeout(1500);

  // Set theme to Light (cycle if needed)
  await page.evaluate(() => window.scrollTo(0, 0));
  for (let i = 0; i < 3; i++) {
    const st = await page.evaluate(() => {
      const tl = document.querySelector(".theme-toggle")?.getAttribute("title") || "";
      const dark = document.documentElement.classList.contains("dark");
      return { isSystem: tl.includes("跟随系统"), isDark: dark, label: tl };
    });
    if (!st.isSystem && !st.isDark) break;
    await page.click(".theme-toggle");
    await page.waitForTimeout(700);
  }
  // Scroll to editor, click Run
  await page.evaluate(() => { const e = document.querySelector(".code-runner-editor .cm-editor"); if (e) e.scrollIntoView({ block: "start" }); });
  await page.waitForTimeout(300);
  await page.click("button:has-text(\"运行\")");
  await page.waitForSelector(".xterm", { timeout: 10000 });
  await page.waitForTimeout(2000);
  console.log("[info] xterm mounted, code executed");

  // Scroll xterm output into view (center of viewport)
  await page.evaluate(() => {
    const xterm = document.querySelector(".xterm-scrollable-element") || document.querySelector(".xterm");
    if (xterm) xterm.scrollIntoView({ block: "center" });
  });
  await page.waitForTimeout(300);

  // Find xterm output area + a control point equidistant from toggle origin.
  // Toggle is at top of page (y≈40), but after scrolling it may be off-screen.
  // VT animation uses toggle coords as circle origin regardless of scroll position
  // (coords are viewport-relative from getBoundingClientRect). After scrollIntoView,
  // toggle scrolls off top — its rect.top becomes negative. The VT circle origin
  // will be at that negative y, meaning the circle starts above the viewport.
  // For equidistant sampling: pick control point at same distance from toggle as xterm,
  // placed horizontally adjacent (same scroll position, different x).
  const geom = await page.evaluate(() => {
    const xterm = document.querySelector(".xterm-scrollable-element") || document.querySelector(".xterm");
    const toggle = document.querySelector(".theme-toggle");
    if (!xterm || !toggle) return null;
    const xr = xterm.getBoundingClientRect();
    const tr = toggle.getBoundingClientRect();
    // xterm sample: right edge, mid-height (pure bg, avoid text on left)
    const xx = xr.right - 10;
    const xy = xr.top + xr.height / 2;
    const tx = tr.left + tr.width / 2;
    const ty = tr.top + tr.height / 2;
    const dist = Math.hypot(xx - tx, xy - ty);
    // control: same distance from toggle, placed to the LEFT of xterm at same y
    // (horizontal mirror preserves distance if toggle is centered; approximate)
    // Better: place control at angle that keeps it in viewport. Use x = tx - (xx-tx),
    // y = xy (mirror across toggle x). Distance = same.
    const cx = Math.round(tx - (xx - tx));
    const cy = Math.round(xy);
    return {
      xterm: { x: Math.round(xx), y: Math.round(xy) },
      toggle: { x: Math.round(tx), y: Math.round(ty) },
      control: { x: cx, y: cy },
      dist: Math.round(dist),
    };
  });
  if (!geom) { console.log("[fatal] no xterm found"); await browser.close(); return; }
  console.log("[info] xterm sample:", geom.xterm, "control:", geom.control, "dist:", geom.dist);

  async function sample(x, y) {
    const png = await page.screenshot({ clip: { x, y, width: 1, height: 1 }, omitBackground: false });
    const px = decodePng(png);
    return { ...px, hex: hex(px) };
  }

  const preX = await sample(geom.xterm.x, geom.xterm.y);
  const preC = await sample(geom.control.x, geom.control.y);
  console.log("[info] baseline xterm:", preX.hex, "control:", preC.hex);

  // Trigger VT (Light → Dark) via __startThemeTransition
  const frames = [];
  const t0 = Date.now();
  await page.evaluate((coords) => {
    window.__startThemeTransition(coords.x, coords.y);
  }, geom.toggle);
  while (Date.now() - t0 < 1500) {
    const t = Date.now() - t0;
    const x = await sample(geom.xterm.x, geom.xterm.y);
    const c = await sample(geom.control.x, geom.control.y);
    frames.push({ t, xterm: x.hex, control: c.hex });
  }

  await page.waitForTimeout(300);
  const postX = await sample(geom.xterm.x, geom.xterm.y);
  const postC = await sample(geom.control.x, geom.control.y);

  await browser.close();

  console.log("\n========== 基线 ==========");
  console.log("  xterm  :", preX.hex, "  control:", preC.hex);
  console.log("\n========== 终态 ==========");
  console.log("  xterm  :", postX.hex, "  control:", postC.hex);

  console.log("\n========== 逐帧时间线 ==========");
  console.log("t(ms)".padEnd(8) + "xterm".padEnd(12) + "control".padEnd(12));
  let lastT = -100;
  for (const f of frames) {
    if (f.t - lastT < 40) continue;
    lastT = f.t;
    console.log(String(f.t).padEnd(8) + f.xterm.padEnd(12) + f.control.padEnd(12));
  }

  function firstChange(key, start) {
    for (const f of frames) if (f[key] !== start) return f.t;
    return -1;
  }
  const tX = firstChange("xterm", preX.hex);
  const tC = firstChange("control", preC.hex);
  console.log("\n========== 判定 ==========");
  console.log(`xterm   首次跳变: t=${tX}ms (${preX.hex} -> ${postX.hex})`);
  console.log(`control 首次跳变: t=${tC}ms (${preC.hex} -> ${postC.hex})`);
  if (tX < 0 || tC < 0) {
    console.log("? 未观察到完整跳变");
  } else {
    const lag = Math.abs(tX - tC);
    console.log(lag <= 50
      ? `✓ xterm 与等距对照点同帧变色(lag=${lag}ms):终端区域参与 VT 动画。`
      : `✗ ${tX > tC ? "xterm" : "control"} 滞后 ${lag}ms:终端区域与界面不同步。`);
  }
}

main().catch((e) => { console.error("[fatal]", e); process.exit(1); });
