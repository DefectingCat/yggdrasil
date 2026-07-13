// VT 主题切换动画逐帧像素采样器。
//
// 验证目标:暗/亮切换的圆形展开 VT 动画对 xterm 终端区域是否生效。
// 预期 bug:xterm canvas 背景在 NEW 快照里仍是旧色,圆形扫过时终端区域
// 「不动」,动画结束后才瞬切;而 CSS 变量驱动的 body-probe / cssvar-probe
// 应随圆形同步变色。
//
// 做法:
// 1. 起一个静态文件服务,serve 项目 public/ + harness.html(用真实的
//    yggdrasil-core.js + terminal.js 构建产物)。
// 2. Playwright 打开 harness,挂载真实 xterm。
// 3. 在点击 toggle 按钮的瞬间启动逐帧采样循环:每帧(rAF 节拍)对三个
//    探针的中心点做 page.screenshot({clip: 1×1}),PNG 解码读像素。
// 4. 采样持续 ~1s(覆盖 0.4s 动画 + 余量),打印每帧三点的 RGB 时间线。
// 5. 判定:比较「动画进行中(mid 帧)」与「动画前(pre)」「动画后(post)」
//    各探针的颜色变化轨迹。
//
// PNG 解码:对 1×1 截图自实现一个最小解码器(过滤 IEND 之前的 IDAT,
// 解 zlib,做 PNG 过滤逆运算)。1×1 的 IDAT 极小,这套解码器足够。
// 不引第三方依赖(避免给项目加 devDep)。
//
// 运行:node scripts/vt-theme-sampler.mjs
// (依赖 npx playwright,chromium 已装在 ~/Library/Caches/ms-playwright)

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { existsSync, readdirSync } from "node:fs";
import { extname, join, normalize, resolve as pathResolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { inflateSync } from "node:zlib";
import { execFileSync } from "node:child_process";

const __require = createRequire(import.meta.url);

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const REPO_ROOT = pathResolve(__dirname, "..");
const PUBLIC_DIR = join(REPO_ROOT, "public");
const HARNESS = join(__dirname, "vt-theme-harness.html");

// ---------- 静态文件服务 ----------
const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".map": "application/json",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
};

function startStaticServer() {
  return new Promise((startResolve) => {
    const server = createServer(async (req, res) => {
      try {
        // 把 URL 映射到 public/ 下;harness.html 单独映射到根。
        let urlPath = decodeURIComponent(req.url.split("?")[0]);
        let filePath;
        if (urlPath === "/" || urlPath === "/index.html") {
          filePath = HARNESS;
        } else {
          // 防路径穿越
          const safe = normalize(urlPath).replace(/^(\.\.[/\\])+/, "");
          filePath = join(PUBLIC_DIR, safe);
        }
        const data = await readFile(filePath);
        res.writeHead(200, {
          "Content-Type": MIME[extname(filePath)] ?? "application/octet-stream",
        });
        res.end(data);
      } catch (e) {
        res.writeHead(404, { "Content-Type": "text/plain" });
        res.end("404: " + req.url + " (" + (e.code || e.message) + ")");
      }
    });
    server.listen(0, "127.0.0.1", () => startResolve(server));
  });
}

// ---------- 最小 PNG 解码器(仅支持 8-bit RGBA/RGB,单或多个 IDAT) ----------
// 对 1×1 截图足够;不做完整 PNG 规范,只覆盖 Playwright 输出格式。
function decodePng(buf) {
  // 验证签名
  const SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (buf.subarray(0, 8).toString("hex") !== SIG.toString("hex")) {
    throw new Error("not a PNG");
  }
  let off = 8;
  let width = 0, height = 0, bitDepth = 0, colorType = 0;
  const idatChunks = [];
  while (off < buf.length) {
    const len = buf.readUInt32BE(off); off += 4;
    const type = buf.toString("ascii", off, off + 4); off += 4;
    const data = buf.subarray(off, off + len); off += len;
    off += 4; // CRC
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === "IDAT") {
      idatChunks.push(data);
    } else if (type === "IEND") {
      break;
    }
  }
  const inflated = inflateSync(Buffer.concat(idatChunks));
  // 计算每像素字节数
  const channels =
    colorType === 6 ? 4 : colorType === 2 ? 3 : colorType === 0 ? 1 : 4;
  const bpp = channels; // 8-bit only
  const stride = width * bpp;
  const raw = Buffer.alloc(height * stride);
  let prevLine = Buffer.alloc(stride); // 上一行(初始全 0)
  let inOff = 0;
  for (let y = 0; y < height; y++) {
    const filter = inflated[inOff++];
    const line = inflated.subarray(inOff, inOff + stride);
    inOff += stride;
    const out = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const cur = line[x];
      const left = x >= bpp ? out[x - bpp] : 0;
      const up = prevLine[x];
      const upLeft = x >= bpp ? prevLine[x - bpp] : 0;
      let v;
      switch (filter) {
        case 0: v = cur; break;            // None
        case 1: v = (cur + left) & 0xff; break;   // Sub
        case 2: v = (cur + up) & 0xff; break;     // Up
        case 3: v = (cur + ((left + up) >> 1)) & 0xff; break; // Average
        case 4: v = (cur + paeth(left, up, upLeft)) & 0xff; break; // Paeth
        default: throw new Error("unknown filter " + filter);
      }
      out[x] = v;
    }
    out.copy(raw, y * stride);
    prevLine = out;
  }
  // 取 (0,0) 像素(我们的 clip 是 1×1,所以 width=height=1)
  return { r: raw[0], g: raw[1], b: raw[2], a: channels === 4 ? raw[3] : 255, width, height };
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

// inflateSync 直接用 node:zlib 的同步解压(1×1 PNG 的 IDAT 极小,同步足够)
// (留此注释说明:曾误用异步 inflate,已改回 inflateSync)

// ---------- 颜色工具 ----------
function rgbHex({ r, g, b }) {
  return (
    "#" +
    [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")
  );
}

// 期望色(取自 input.css,与 harness 内联变量一致)
const EXPECT = {
  light: { theme: "#eff1f5", codeblock: "#dce0e8" },
  dark: { theme: "#1e1e2e", codeblock: "#313244" },
};

// ---------- 定位 playwright ----------
// playwright 不在本项目 deps 里(避免给项目加 devDep)。从 npx 缓存里找。
// 用 createRequire 加载 CJS 版(playwright-core 的 chromium 是延迟赋值,
// ESM 动态 import 的命名导出快照拿不到它,必须走 require)。
function loadPlaywrightChromium() {
  const candidates = [];
  // 1. npx 缓存(每个 hash 一个隔离 node_modules)
  const npxCache = join(homedir(), ".npm", "_npx");
  if (existsSync(npxCache)) {
    for (const hash of readdirSync(npxCache)) {
      candidates.push(join(npxCache, hash, "node_modules", "playwright-core"));
      candidates.push(join(npxCache, hash, "node_modules", "playwright"));
    }
  }
  // 2. 全局 node_modules
  let globalRoot = "";
  try {
    globalRoot = execFileSync("npm", ["root", "-g"], { encoding: "utf-8" }).trim();
  } catch {
    // npm 不在 PATH,跳过
  }
  if (globalRoot) {
    candidates.push(join(globalRoot, "playwright-core"));
    candidates.push(join(globalRoot, "playwright"));
  }
  // 3. 项目内(以防将来加了 dep)
  candidates.push(join(REPO_ROOT, "node_modules", "playwright-core"));
  candidates.push(join(REPO_ROOT, "node_modules", "playwright"));
  candidates.push(join(REPO_ROOT, "libs", "node_modules", "playwright-core"));
  candidates.push(join(REPO_ROOT, "libs", "node_modules", "playwright"));

  for (const dir of candidates) {
    if (!existsSync(dir)) continue;
    try {
      const require = __require;
      const pw = require(dir);
      if (pw?.chromium?.launch) {
        return pw.chromium;
      }
      // playwright 包(非 core):chromium 在 default 里
      if (pw?.default?.chromium?.launch) {
        return pw.default.chromium;
      }
    } catch {
      // 试下一个
    }
  }
  throw new Error(
    "找不到 playwright。请先运行: npx playwright@latest install chromium"
  );
}

// ---------- 主流程 ----------
async function main() {
  const chromium = loadPlaywrightChromium();
  console.log("[info] playwright chromium loaded");

  const server = await startStaticServer();
  const port = server.address().port;
  const url = `http://127.0.0.1:${port}/`;
  console.log(`[info] serving on ${url}`);

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 600, height: 400 },
    deviceScaleFactor: 1, // 1:1 像素,避免 retina 缩放干扰
  });
  const page = await context.newPage();
  // VT 动画需要 reduced-motion 关闭。显式设为 no-preference,
  // 避免被系统偏好带偏(系统若开了 reduce 则 __startThemeTransition 会走无动画分支)。
  await page.emulateMedia({ reducedMotion: "no-preference" });
  page.on("console", (m) => {
    if (m.type() === "error") console.log("[page error]", m.text());
  });

  await page.goto(url, { waitUntil: "networkidle" });
  // 等 xterm 挂载
  await page.waitForFunction(() => window.__harnessReady === true, { timeout: 5000 });
  console.log("[info] xterm mounted, ready");

  // 探针中心坐标(与 harness .probe 的 top/left + 30 居中一致;probe 60×60)。
  // xterm 采样点取右下角(left+50, top+50)避开文字(DOM renderer 文字从左上排开)。
  const probes = {
    body: { x: 200 + 30, y: 120 + 30 },
    cssvar: { x: 270 + 30, y: 120 + 30 },
    xterm: { x: 340 + 50, y: 120 + 50 }, // 右下角,避开文字
  };

  async function samplePoint(name, x, y) {
    const png = await page.screenshot({
      clip: { x, y, width: 1, height: 1 },
      omitBackground: false,
    });
    const px = decodePng(png);
    return { name, ...px, hex: rgbHex(px) };
  }

  async function sampleAll(label) {
    const out = {};
    for (const [k, p] of Object.entries(probes)) {
      out[k] = await samplePoint(k, p.x, p.y);
    }
    return { label, t: Date.now(), ...out };
  }

  // ---- baseline:light 态 ----
  const pre = await sampleAll("pre(light)");

  // ---- 触发 VT 动画并逐帧采样 ----
  // 点击 toggle 按钮(按钮在左上,圆形从按钮中心向右展开扫过探针)。
  // harness 的 click handler 会:① 调 __startThemeTransition(同步 VT)
  // ② setTimeout(0) 调 set_theme(模拟 Dioxus use_effect 延迟)。
  // 采样在点击后立即开始,紧密循环 ~1.2s 覆盖 0.4s 动画 + set_theme 后续。
  const frames = [];
  // 记录点击时刻(用 performance.now 在 page 内打点,避免 round-trip 偏差)
  await page.evaluate(() => { window.__clickAt = performance.now(); });
  const t0Real = Date.now();
  await page.click("#toggle-btn");

  // 紧密采样 ~1.2s
  const SAMPLE_MS = 1200;
  while (Date.now() - t0Real < SAMPLE_MS) {
    const t = Date.now() - t0Real;
    const f = await sampleAll(String(t));
    f.t = t;
    frames.push(f);
  }

  // ---- 终态 ----
  await page.waitForTimeout(300); // 确保动画完全结束 + vt.finished 清理
  const post = await sampleAll("post(dark)");
  // 调试:确认 .dark class 与 xterm inline bg 的最终状态
  const dbgFinal = await page.evaluate(() => {
    const html = document.documentElement;
    const el = document.querySelector("#xterm-mount .xterm-scrollable-element");
    return {
      htmlHasDark: html.classList.contains("dark"),
      xtermInlineBg: el ? el.style.backgroundColor : "(no element)",
      setThemeCalled: !!window.__setThemeCalledAt,
    };
  });
  console.log("[debug] final:", JSON.stringify(dbgFinal));

  await browser.close();
  server.close();

  // ---------- 分析 ----------
  console.log("\n========== 基线(light) ==========");
  for (const k of ["body", "cssvar", "xterm"]) {
    const v = pre[k];
    console.log(`  ${k.padEnd(7)} ${v.hex}  (r=${v.r} g=${v.g} b=${v.b})`);
  }
  console.log("\n========== 终态(dark) ==========");
  for (const k of ["body", "cssvar", "xterm"]) {
    const v = post[k];
    console.log(`  ${k.padEnd(7)} ${v.hex}  (r=${v.r} g=${v.g} b=${v.b})`);
  }

  // 抽帧打印时间线(每 ~50ms 取一帧,避免刷屏)
  console.log("\n========== 逐帧时间线 ==========");
  console.log(
    "t(ms)".padEnd(8) +
      "body".padEnd(10) +
      "cssvar".padEnd(10) +
      "xterm".padEnd(10)
  );
  let lastT = -100;
  for (const f of frames) {
    if (f.t - lastT < 45) continue; // ~45ms 一帧
    lastT = f.t;
    console.log(
      String(f.t).padEnd(8) +
        f.body.hex.padEnd(10) +
        f.cssvar.hex.padEnd(10) +
        f.xterm.hex.padEnd(10)
    );
  }

  // ---------- 判定 ----------
  // 三个探针的「首次跳变时刻」(相对 t0Real,即点击后多久颜色变了):
  //   - body / cssvar:CSS 变量驱动,VT 的 NEW 快照里已是 dark,圆形扫过采样点时
  //     从 light 瞬跳 dark。应在动画窗口(~400ms)内发生(取决于圆形半径何时覆盖探针)。
  //   - xterm:修复前,背景 inline style 不随 .dark 翻转,set_theme 在 VT 回调后
  //     异步跑 → NEW 快照里 xterm 仍是 light,圆形扫过看不到变化,动画后才瞬切。
  //     修复后,VT 回调内 dispatch 事件 → xterm 同步 setTheme → NEW 快照里已是 dark,
  //     与 body 同帧跳变。
  // 判据:xterm 跳变时刻 - body 跳变时刻 的差值(lag)。
  //   - lag < 15ms(同帧):动画对 xterm 生效,修复成功。
  //   - lag > 15ms:xterm 在 VT 动画期间保持旧色,修复未生效。
  function transitionFrame(key) {
    const startHex = pre[key].hex;
    const endHex = post[key].hex;
    if (startHex === endHex) return -1; // pre==post,全程无变化
    for (let i = 0; i < frames.length; i++) {
      if (frames[i][key].hex !== startHex) return frames[i].t;
    }
    return -2; // 帧序列里没观察到跳变(但 pre≠post,说明跳变在采样窗口外)
  }
  const tBody = transitionFrame("body");
  const tCss = transitionFrame("cssvar");
  const tXterm = transitionFrame("xterm");

  console.log("\n========== 判定 ==========");
  console.log(`body   首次跳变: t=${tBody}ms  (pre=${pre.body.hex} → post=${post.body.hex})`);
  console.log(`cssvar 首次跳变: t=${tCss}ms  (pre=${pre.cssvar.hex} → post=${post.cssvar.hex})`);
  console.log(`xterm  首次跳变: t=${tXterm}ms (pre=${pre.xterm.hex} → post=${post.xterm.hex})`);

  // 判据:比较 xterm 与 cssvar 的跳变时刻(两者几何距离相近,圆形同时覆盖)。
  // - 若同帧(lag < 15ms):xterm 与 CSS 变量驱动的 cssvar 同步变色 → 修复成功,
  //   xterm 的 inline bg 已进入 NEW 快照,圆形展开对终端区域生效。
  // - 若 xterm 明显晚于 cssvar:xterm 在 NEW 快照里仍是旧色 → 修复未生效。
  // (不与 body 比:body 离 toggle 原点更近,圆形更早覆盖,几何延迟会污染判据。)
  let verdict, detail;
  if (tXterm === -1) {
    verdict = "?";
    detail = `xterm 全程未变色(pre=${pre.xterm.hex}==post=${post.xterm.hex}),set_theme 未生效或采样点未命中纯背景区。`;
  } else if (tCss < 0) {
    verdict = "?";
    detail = "cssvar 未观察到跳变,无法建立对照基准。";
  } else {
    const lag = tXterm - tCss;
    if (Math.abs(lag) > 15) {
      verdict = "✗";
      detail = `BUG 仍存在:xterm 比 cssvar ${lag > 0 ? "晚" : "早"} ${Math.abs(lag)}ms 变色。`;
      detail += `cssvar 在 t=${tCss}ms 变色,xterm 在 t=${tXterm}ms——两者几何等距却不同步,`;
      detail += `说明 xterm 的 inline bg 未进入 NEW 快照。`;
    } else {
      verdict = "✓";
      detail = `修复成功:xterm 与 cssvar 同帧变色(lag=${lag}ms,均 t=${tCss}ms)。`;
      detail += `VT 回调内的事件让 xterm 在 NEW 快照前同步 setTheme,`;
      detail += `圆形展开对终端区域生效(cssvar 是 CSS 变量驱动的等距对照点)。`;
    }
  }

  console.log("\n---------- 结论 ----------");
  console.log(verdict + " " + detail);
}

main().catch((e) => {
  console.error("[fatal]", e);
  process.exit(1);
});
