// 真实页面 VT 主题动画采样器 —— 针对 dx serve 运行中的真实 app。
//
// 目的:在真实 Dioxus 环境里验证「可运行代码块颜色是否随 VT 圆形展开动画变色」,
// 还是像用户报告的那样「直接瞬切,不受 VT 动画控制」。
//
// 做法:
// 1. 打开 http://localhost:8080/(首页有 runnable 代码块)。
// 2. 找到 CodeMirror 编辑器容器(.code-runner-editor)和主题切换按钮(.theme-toggle)。
// 3. 记录编辑器中心点 + 一个普通页面元素(div.bg-paper 之类)的中心点作为对照。
// 4. 点击主题按钮,启动逐帧采样循环(~1.2s),每帧对两个点截图读像素。
// 5. 打印时间线 + 判定:编辑器是否与对照点同帧变色(修复成功),还是滞后/瞬切(bug)。
//
// 运行:node scripts/vt-real-page-sampler.mjs
// 前提:make dev 已启动,首页有 runnable 代码块。

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { inflateSync } from "node:zlib";

const __require = createRequire(import.meta.url);
const REPO_ROOT = __require("node:path").resolve(
  __require("node:url").fileURLToPath(new URL(".", import.meta.url)),
  "..",
);

function loadPlaywrightChromium() {
  const candidates = [];
  const npxCache = join(homedir(), ".npm", "_npx");
  if (existsSync(npxCache)) {
    for (const hash of readdirSync(npxCache)) {
      candidates.push(join(npxCache, hash, "node_modules", "playwright-core"));
      candidates.push(join(npxCache, hash, "node_modules", "playwright"));
    }
  }
  candidates.push(join(REPO_ROOT, "node_modules", "playwright-core"));
  candidates.push(join(REPO_ROOT, "node_modules", "playwright"));
  for (const dir of candidates) {
    if (!existsSync(dir)) continue;
    try {
      const pw = __require(dir);
      if (pw?.chromium?.launch) return pw.chromium;
      if (pw?.default?.chromium?.launch) return pw.default.chromium;
    } catch {}
  }
  throw new Error("找不到 playwright。运行: npx playwright@latest install chromium");
}

// ---------- PNG 解码(1×1) ----------
function decodePng(buf) {
  const SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (buf.subarray(0, 8).toString("hex") !== SIG.toString("hex")) throw new Error("not PNG");
  let off = 8;
  let w = 0, h = 0, ct = 0;
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
  const ch = ct === 6 ? 4 : ct === 2 ? 3 : ct === 0 ? 1 : 4;
  const bpp = ch;
  const stride = w * bpp;
  let prev = Buffer.alloc(stride);
  let io = 0;
  const raw = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = inf[io++];
    const line = inf.subarray(io, io + stride);
    io += stride;
    const out = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const c = line[x];
      const l = x >= bpp ? out[x - bpp] : 0;
      const u = prev[x];
      const ul = x >= bpp ? prev[x - bpp] : 0;
      let v;
      switch (f) {
        case 0: v = c; break;
        case 1: v = (c + l) & 0xff; break;
        case 2: v = (c + u) & 0xff; break;
        case 3: v = (c + ((l + u) >> 1)) & 0xff; break;
        case 4: {
          const p = l + u - ul;
          const pa = Math.abs(p - l), pb = Math.abs(p - u), pc = Math.abs(p - ul);
          v = (c + (pa <= pb && pa <= pc ? l : pb <= pc ? u : ul)) & 0xff;
          break;
        }
        default: throw new Error("filter " + f);
      }
      out[x] = v;
    }
    out.copy(raw, y * stride);
    prev = out;
  }
  return { r: raw[0], g: raw[1], b: raw[2] };
}

function hex({ r, g, b }) {
  return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
}

async function main() {
  const chromium = loadPlaywrightChromium();
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 900 },
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();
  await page.emulateMedia({ reducedMotion: "no-preference" });

  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });

  console.log("[info] navigating to http://localhost:8080/post/rust");
  await page.goto("http://localhost:8080/post/rust", { waitUntil: "networkidle", timeout: 30000 });

  // 等 CodeMirror 编辑器挂载
  console.log("[info] waiting for CodeMirror editor...");
  try {
    await page.waitForSelector(".code-runner-editor .cm-editor", { timeout: 10000 });
  } catch {
    console.log("[warn] 没找到 .code-runner-editor .cm-editor,首页可能没有 runnable 代码块");
    await browser.close();
    return;
  }
  // 额外等一帧确保 CodeMirror 完全渲染
  await page.waitForTimeout(500);

  // 滚动到第一个编辑器,使其在视口内可见
  await page.evaluate(() => {
    const editor = document.querySelector(".code-runner-editor .cm-editor");
    if (editor) editor.scrollIntoView({ block: "center" });
  });
  await page.waitForTimeout(300);

  // 找到编辑器中心点 + 主题按钮中心点 + 一个普通背景对照点
  // 采样坐标基于 scrollIntoView 后的 getBoundingClientRect(视口相对)
  const geom = await page.evaluate(() => {
    const editor = document.querySelector(".code-runner-editor .cm-editor");
    const toggle = document.querySelector(".theme-toggle");
    const er = editor.getBoundingClientRect();
    const tr = toggle.getBoundingClientRect();
    // 对照点:编辑器附近的普通页面背景(取编辑器左侧 60px 外,同行高度)
    const ctrlX = Math.max(20, er.left - 60);
    const ctrlY = er.top + er.height / 2;
    return {
      editor: { x: Math.round(er.left + er.width / 2), y: Math.round(er.top + er.height / 2) },
      toggle: { x: Math.round(tr.left + tr.width / 2), y: Math.round(tr.top + tr.height / 2) },
      control: { x: Math.round(ctrlX), y: Math.round(ctrlY) },
      editorRect: { left: er.left, top: er.top, w: er.width, h: er.height },
    };
  });
  console.log("[info] editor center:", geom.editor, "toggle center:", geom.toggle, "control:", geom.control);

  // 当前主题
  const isDarkBefore = await page.evaluate(() => document.documentElement.classList.contains("dark"));
  console.log("[info] current theme (dark?):", isDarkBefore);

  // 主题循环:System → Light → Dark → System。
  // 我们需要一次「实际明暗翻转」来触发 VT 动画。先预点击把主题设到 Light(若当前
  // 是 System 或 Dark),确保下一次点击是 Light→Dark 的真实翻转。
  if (isDarkBefore) {
    // Dark → System(light resolved):先翻到浅色基线
    await page.click(".theme-toggle");
    await page.waitForTimeout(800);
  }
  // 现在确保是 Light(若仍 System,再点一次到 Light)
  const themeLabel1 = await page.evaluate(() => document.querySelector(".theme-toggle")?.getAttribute("title") || "");
  if (themeLabel1.includes("跟随系统")) {
    await page.click(".theme-toggle");
    await page.waitForTimeout(800);
  }
  // 滚动回顶部让 toggle 可见,再滚回编辑器位置准备采样
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(100);
  // 确认现在是 Light(浅色,dark class 无)
  const confirmedLight = await page.evaluate(() => !document.documentElement.classList.contains("dark"));
  console.log("[info] baseline set to Light:", confirmedLight);

  // 滚回编辑器位置
  await page.evaluate(() => {
    const editor = document.querySelector(".code-runner-editor .cm-editor");
    if (editor) editor.scrollIntoView({ block: "center" });
  });
  await page.waitForTimeout(300);

  // 重新读取坐标(scrollIntoView 后)
  // 关键:对照点必须与编辑器到 toggle 原点等距,否则圆形展开到达两者的时间
  // 不同(几何延迟),会被误判为 bug。对照点取编辑器正上方/下方等距的页面背景。
  const geom2 = await page.evaluate(() => {
    const editor = document.querySelector(".code-runner-editor .cm-editor");
    const toggle = document.querySelector(".theme-toggle");
    const er = editor.getBoundingClientRect();
    const tr = toggle.getBoundingClientRect();
    const ex = er.left + er.width / 2;
    const ey = er.top + er.height / 2;
    const tx = tr.left + tr.width / 2;
    const ty = tr.top + tr.height / 2;
    const distEditor = Math.hypot(ex - tx, ey - ty);
    // 对照点:在 toggle 正下方(同 x),距离 = 编辑器到 toggle 的距离
    // 这样对照点与编辑器等距,圆形同时覆盖两者
    const ctrlX = Math.round(tx);
    const ctrlY = Math.round(ty + distEditor);
    // 确保对照点在视口内且在页面背景上(非编辑器区域)
    return {
      editor: { x: Math.round(ex), y: Math.round(ey) },
      control: { x: ctrlX, y: ctrlY },
      distEditor: Math.round(distEditor),
    };
  });
  Object.assign(geom, geom2);
  console.log("[info] editor center:", geom.editor, "control:", geom.control, "dist:", geom.distEditor);

  async function sample(x, y) {
    const png = await page.screenshot({ clip: { x, y, width: 1, height: 1 }, omitBackground: false });
    const px = decodePng(png);
    return { ...px, hex: hex(px) };
  }

  // baseline (Light)
  const preEditor = await sample(geom.editor.x, geom.editor.y);
  const preControl = await sample(geom.control.x, geom.control.y);
  console.log("[info] baseline editor:", preEditor.hex, "control:", preControl.hex);

  // 点击主题按钮(Light → Dark,真实翻转,触发 VT 动画),开始逐帧采样。
  // toggle 在顶部(y=40),需先滚到顶部点击,再立即滚回编辑器位置采样。
  // 但滚动会打断 VT 动画截图——改用 JS 直接调 __startThemeTransition(toggle 坐标),
  // 这样不需要滚动,且与真实 onclick 行为一致(传 toggle 坐标作圆心)。
  const frames = [];
  const t0 = Date.now();
  // 直接在 page 内调 __startThemeTransition,圆心用 toggle 坐标(顶部),
  // 圆形从顶部展开扫向编辑器。
  await page.evaluate(() => {
    const toggle = document.querySelector(".theme-toggle");
    const r = toggle.getBoundingClientRect();
    const x = r.left + r.width / 2;
    const y = r.top + r.height / 2;
    window.__startThemeTransition(x, y);
  });
  while (Date.now() - t0 < 1500) {
    const t = Date.now() - t0;
    const e = await sample(geom.editor.x, geom.editor.y);
    const c = await sample(geom.control.x, geom.control.y);
    frames.push({ t, editor: e.hex, control: c.hex });
  }

  // 终态
  await page.waitForTimeout(300);
  const postEditor = await sample(geom.editor.x, geom.editor.y);
  const postControl = await sample(geom.control.x, geom.control.y);
  const isDarkAfter = await page.evaluate(() => document.documentElement.classList.contains("dark"));
  // 注意:直接调 __startThemeTransition 只翻 DOM class,不更新 Dioxus theme signal,
  // 所以 toggle 的 title 不变,但 dark class 已翻转——这正是我们要测的 VT 动画效果。

  await browser.close();

  // 分析
  console.log("\n========== 基线 ==========");
  console.log("  editor :", preEditor.hex, "  control:", preControl.hex);
  console.log("\n========== 终态 ==========");
  console.log("  editor :", postEditor.hex, "  control:", postControl.hex);
  console.log("  theme dark?:", isDarkBefore, "->", isDarkAfter);

  console.log("\n========== 逐帧时间线 ==========");
  console.log("t(ms)".padEnd(8) + "editor".padEnd(12) + "control".padEnd(12));
  let lastT = -100;
  for (const f of frames) {
    if (f.t - lastT < 40) continue;
    lastT = f.t;
    console.log(String(f.t).padEnd(8) + f.editor.padEnd(12) + f.control.padEnd(12));
  }

  // 判定:editor 与 control 首次跳变时刻
  function firstChange(key, startHex) {
    for (const f of frames) {
      if (f[key] !== startHex) return f.t;
    }
    return -1;
  }
  const tEditor = firstChange("editor", preEditor.hex);
  const tControl = firstChange("control", preControl.hex);
  console.log("\n========== 判定 ==========");
  console.log(`editor  首次跳变: t=${tEditor}ms  (${preEditor.hex} -> ${postEditor.hex})`);
  console.log(`control 首次跳变: t=${tControl}ms (${preControl.hex} -> ${postControl.hex})`);

  if (tEditor < 0 && tControl < 0) {
    console.log("? 两者均未观察到跳变(可能采样点未命中或主题未翻)");
  } else if (tEditor < 0) {
    console.log("? editor 全程未变色,control 变了——编辑器可能没参与主题切换");
  } else if (tControl < 0) {
    console.log("? control 全程未变色,editor 变了——对照点选错");
  } else {
    const lag = Math.abs(tEditor - tControl);
    if (lag <= 50) {
      console.log(`✓ editor 与对照点同帧变色(lag=${lag}ms):编辑器参与了 VT 动画。`);
    } else {
      const later = tEditor > tControl ? "editor" : "control";
      console.log(`✗ ${later} 滞后 ${lag}ms:编辑器与界面其他部分不同步——`);
      console.log(`  这正是「代码块直接瞬切,不受 VT 动画控制」的表现。`);
    }
  }

  if (errors.length) {
    console.log("\n========== 页面错误 ==========");
    errors.slice(0, 5).forEach((e) => console.log("  " + e));
  }
}

main().catch((e) => {
  console.error("[fatal]", e);
  process.exit(1);
});
