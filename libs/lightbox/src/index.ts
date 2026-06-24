import { fitCentered, transformFor, originalUrl, type Rect } from "./geometry";

interface LightboxState {
  overlay: HTMLDivElement;
  img: HTMLImageElement;
  caption: HTMLElement;
  counter: HTMLDivElement;
  prevBtn: HTMLButtonElement | null;
  nextBtn: HTMLButtonElement | null;
  originNode: HTMLElement;
  gallery: HTMLElement[];
  index: number | null;
  isSingle: boolean;
  openScrollY: number;
  origSrc: string;
  altText: string;
  closing: boolean;
  reduced: boolean;
  scrollHandler: ((this: Window, ev: Event) => void) | null;
  keyHandler: ((this: Document, ev: KeyboardEvent) => void) | null;
  target?: Rect;
  baseW?: number;
  baseH?: number;
}

declare global {
  interface Window {
    __initLightbox: (selectors: string | string[]) => void;
    __lightboxSelectors?: string[];
  }
}

export {};

(function () {
  "use strict";

  // ============ 工具函数 ============

  function prefersReducedMotion(): boolean {
    return (
      !!window.matchMedia &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    );
  }

  // 读取元素当前在视口里的 rect（用于飞行起点/终点）。
  // 统一映射成 {x,y,w,h}：getBoundingClientRect 返回的 DOMRect 用
  // left/top/width/height，而 fitCentered/transformFor 用 x/y/w/h，
  // 这里转成一致格式，避免 .w 读到 undefined。
  function rectOf(el: Element): Rect {
    var r = el.getBoundingClientRect();
    return {
      x: r.left,
      y: r.top,
      w: r.width,
      h: r.height,
    };
  }

  // 注意：fitCentered / transformFor / originalUrl 已抽到 ./geometry.ts（见 Task 2）。

  // ============ 懒加载 ============

  // 为单个 .blur-img 容器初始化高清图懒加载。
  // IO 进入视口后把 data-src 写入 src，加载完成加 is-loaded 触发 CSS 淡入。
  function initLazyLoad(container: Element): void {
    var raw = container.querySelector(".blur-img-full");
    if (!(raw instanceof HTMLImageElement)) return;
    // 用 const + 显式类型锁住窄化结果：var 在闭包内会被放宽回 Element | null，
    // 导致 onFullLoaded/IntersectionObserver 回调里访问 .style/.src 报错。
    var fullImg: HTMLImageElement = raw;
    if (container.getAttribute("data-blur-init")) return;
    container.setAttribute("data-blur-init", "true");

    var rawSrc = fullImg.getAttribute("data-src");
    if (!rawSrc) return;
    // 同上：const 锁住 string 窄化，避免闭包内放宽回 string | null。
    var fullSrc: string = rawSrc;

    var onFullLoaded = function (): void {
      // 给容器加 is-loaded，CSS 据此显式隐藏 placeholder。
      // 直接把 full 层 opacity 设为 1（清掉 transition），不依赖 CSS 的 opacity
      // 过渡：合成层重绘时机不稳定，可能导致 full 层卡在 opacity:0，直到一次
      // 强制重排才更新。
      container.classList.add("is-loaded");
      fullImg.style.transition = "none";
      fullImg.style.opacity = "1";
    };
    fullImg.addEventListener("load", onFullLoaded);
    // 缓存兜底：若设 src 时图片已在缓存（load 几乎立即触发，可能早于监听注册），
    // 用 complete 补一次。注意无 src 的 img complete 也为 true，故先判 src。
    if (fullImg.getAttribute("src") && fullImg.complete) {
      onFullLoaded();
    }

    if ("IntersectionObserver" in window) {
      var io = new IntersectionObserver(
        function (entries: IntersectionObserverEntry[]): void {
          entries.forEach(function (entry): void {
            if (entry.isIntersecting) {
              fullImg.src = fullSrc;
              io.unobserve(container);
            }
          });
        },
        { rootMargin: "200px" }
      );
      io.observe(container);
    } else {
      fullImg.src = fullSrc;
    }
  }

  // ============ 图像收集 ============

  // 收集所有 selectors命中的 .blur-img 节点。
  // gallery: 正文图（组成图集）；singles: 带 lightbox-single class 的单张图（如封面）。
  function collectImages(roots: Element[]): { gallery: HTMLElement[]; singles: HTMLElement[] } {
    var gallery: HTMLElement[] = [];
    var singles: HTMLElement[] = [];
    for (var i = 0; i < roots.length; i++) {
      var nodes = roots[i].querySelectorAll(".blur-img");
      for (var j = 0; j < nodes.length; j++) {
        var n = nodes[j];
        if (!(n instanceof HTMLElement)) continue;
        if (n.classList.contains("lightbox-single")) {
          singles.push(n);
        } else {
          gallery.push(n);
        }
      }
    }
    return { gallery: gallery, singles: singles };
  }

  // ============ 灯箱状态与开/关 ============

  // 当前灯箱状态（同时只允许一个灯箱）。
  var state: LightboxState | null = null;

  function openLightbox(originNode: HTMLElement, gallery: HTMLElement[], index: number | null): void {
    if (state) closeLightbox(true);

    var fullImgEl = originNode.querySelector(".blur-img-full");
    if (!(fullImgEl instanceof HTMLImageElement)) return;
    var dataSrc = fullImgEl.getAttribute("data-src") || "";
    var origSrc = originalUrl(dataSrc);
    var altText = fullImgEl.getAttribute("alt") || "";
    var isSingle =
      originNode.classList.contains("lightbox-single") ||
      gallery.length === 0;

    var vw = window.innerWidth;
    var vh = window.innerHeight;

    // 建 DOM
    var overlay = document.createElement("div");
    overlay.className = "lightbox-overlay";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-modal", "true");
    overlay.setAttribute("aria-label", "图片预览");
    overlay.setAttribute("tabindex", "-1");
    // 创建后立即设 opacity 0：append 到 DOM 时是透明的，避免在图片加载期间
    // （start() 执行前）显示全黑背景造成闪烁。start() 的渐变会把 opacity 升到 1。
    overlay.style.opacity = "0";

    var img = document.createElement("img");
    img.className = "lightbox-img";
    img.setAttribute("alt", altText);
    // 加载前先占 0 尺寸，避免原图（可能数千 px）在加载期间撑大文档
    // 可滚动区、触发非预期的 scroll 事件。start() 拿到 natural 尺寸后再设真实值。
    img.style.width = "0px";
    img.style.height = "0px";

    var caption = document.createElement("figcaption");
    caption.className = "lightbox-caption";
    caption.textContent = altText;
    if (!altText) caption.style.display = "none";

    var counter = document.createElement("div");
    counter.className = "lightbox-counter";
    if (isSingle || gallery.length === 0) {
      counter.style.display = "none";
    } else {
      counter.textContent = (index ?? 0) + 1 + " / " + gallery.length;
    }

    // 图集模式（>1 张）才加左右导航箭头；单张不显示。
    var prevBtn: HTMLButtonElement | null = null;
    var nextBtn: HTMLButtonElement | null = null;
    if (!isSingle && gallery.length > 1) {
      prevBtn = document.createElement("button");
      prevBtn.className = "lightbox-nav lightbox-prev";
      prevBtn.setAttribute("type", "button");
      prevBtn.setAttribute("aria-label", "上一张");
      prevBtn.textContent = "\u2039";

      nextBtn = document.createElement("button");
      nextBtn.className = "lightbox-nav lightbox-next";
      nextBtn.setAttribute("type", "button");
      nextBtn.setAttribute("aria-label", "下一张");
      nextBtn.textContent = "\u203a";
    }

    overlay.appendChild(img);
    overlay.appendChild(caption);
    overlay.appendChild(counter);
    if (prevBtn) overlay.appendChild(prevBtn);
    if (nextBtn) overlay.appendChild(nextBtn);
    document.body.appendChild(overlay);

    state = {
      overlay: overlay,
      img: img,
      caption: caption,
      counter: counter,
      prevBtn: prevBtn,
      nextBtn: nextBtn,
      originNode: originNode,
      gallery: gallery,
      index: index,
      isSingle: isSingle,
      openScrollY: window.scrollY,
      origSrc: origSrc,
      altText: altText,
      closing: false,
      reduced: prefersReducedMotion(),
      scrollHandler: null,
      keyHandler: null,
    };

    // 焦点移入灯箱
    overlay.focus();
    // 立即绑定交互（不等图片加载）：加载期间 Esc/滚动/点背景也须可关闭。
    bindInteractions();

    // 图片加载后再做动画（naturalW/H 要等加载）
    var start = function (): void {
      if (!state) return; // 加载前可能已被关闭
      var naturalW = img.naturalWidth || img.clientWidth || 1;
      var naturalH = img.naturalHeight || img.clientHeight || 1;
      var originRect = rectOf(originNode);

      // 基准 = originRect（文章里图片的实际尺寸）。
      // img 的布局尺寸固定为 originRect，transform 的 scale 相对它缩放：
      // 首帧（文章位置）scale=1，居中态 scale=target.w/originRect.w。
      // 这样无论 target 比 originRect 大或小，动画都是「从文章图原样连续缩放」，
      // 视觉上是原地展开，不会像「从外面飞来」。灯箱图尺寸恒为视口最大（fitCentered）。
      var target = fitCentered(naturalW, naturalH, vw, vh);
      var baseW = originRect.w;
      var baseH = originRect.h;
      // 存基准与目标，供关闭/滚动关闭复用同一对（保证 scale 连续）。
      state.target = target;
      state.baseW = baseW;
      state.baseH = baseH;
      img.style.width = baseW + "px";
      img.style.height = baseH + "px";

      // reduced-motion：直接淡入居中
      if (state.reduced) {
        img.style.opacity = "0";
        img.style.transform = transformFor(target, baseW, baseH);
        img.style.left = "0";
        img.style.top = "0";
        overlay.style.opacity = "0";
        // 下一帧淡入
        requestAnimationFrame(function (): void {
          if (!state) return;
          overlay.style.transition = "opacity 200ms ease-out";
          img.style.transition = "opacity 200ms ease-out";
          overlay.style.opacity = "1";
          img.style.opacity = "1";
        });
        return;
      }

      // 首帧：文章位置 + 原尺寸（scale=1），透明，且关闭 transition
      img.style.transition = "none";
      img.style.left = "0";
      img.style.top = "0";
      img.style.transform = transformFor(originRect, baseW, baseH);
      img.style.opacity = "0";
      overlay.style.opacity = "0";
      // 强制 reflow，确保首帧的 transform 已提交到渲染层。
      // 否则单层 rAF 里浏览器可能合并首帧与目标帧，动画从错误位置起跳。
      void img.offsetHeight;

      // double-rAF：第一帧绘制首帧（无动画），第二帧才启动 transition 到居中。
      requestAnimationFrame(function (): void {
        if (!state) return;
        requestAnimationFrame(function (): void {
          if (!state) return;
          img.style.transition =
            "transform 250ms ease-out, opacity 250ms ease-out";
          overlay.style.transition = "opacity 250ms ease-out";
          img.style.transform = transformFor(target, baseW, baseH);
          img.style.opacity = "1";
          overlay.style.opacity = "1";
        });
      });
    };

    if (img.complete && img.naturalWidth) {
      start();
    } else {
      img.addEventListener("load", start, { once: true });
    }
    img.src = origSrc;
  }

  function closeLightbox(immediate: boolean): void {
    if (!state || state.closing) return;
    state.closing = true;
    cleanupInteractions();

    var s = state;

    // 基准 = originRect 尺寸（与打开时一致），scale 相对它缩放。
    var baseW = s.baseW || (s.target ? s.target.w : 1);
    var baseH = s.baseH || (s.target ? s.target.h : 1);
    var originRect = rectOf(s.originNode); // 实时读，处理期间滚动过的情况

    if (s.reduced || immediate) {
      removeOverlay();
      return;
    }

    // 飞回 originRect：scale 从 1 缩到 originRect.w/baseW
    s.img.style.transition =
      "transform 250ms ease-out, opacity 250ms ease-out";
    s.overlay.style.transition = "opacity 250ms ease-out";
    s.img.style.transform = transformFor(originRect, baseW, baseH);
    s.img.style.opacity = "0";
    s.overlay.style.opacity = "0";

    var done = function (): void {
      removeOverlay();
    };
    // 250ms 兜底，避免 transitionend 不触发
    var timer = setTimeout(done, 280);
    s.img.addEventListener("transitionend", function (): void { clearTimeout(timer); done(); }, { once: true });
  }

  function removeOverlay(): void {
    if (!state) return;
    var prev = state.originNode;
    if (state.overlay && state.overlay.parentNode) {
      state.overlay.parentNode.removeChild(state.overlay);
    }
    state = null;
    // 焦点归还：.blur-img 是 span 不可聚焦，让其内部 full img 获得焦点
    if (prev) {
      var f = prev.querySelector(".blur-img-full");
      if (f instanceof HTMLImageElement) {
        f.setAttribute("tabindex", "-1");
        f.focus();
      }
    }
  }

  // ============ 图集切换 ============

  // 图集切换：淡入淡出，不飞行。newIndex 循环（首尾衔接）。
  function gotoIndex(newIndex: number): void {
    if (!state || state.isSingle) return;
    var s = state;
    if (!s.gallery || s.gallery.length === 0) return;
    if (newIndex < 0) newIndex = s.gallery.length - 1;
    if (newIndex >= s.gallery.length) newIndex = 0;
    if (newIndex === s.index) return;

    var newNode = s.gallery[newIndex];
    var fullImgEl = newNode.querySelector(".blur-img-full");
    if (!(fullImgEl instanceof HTMLImageElement)) return;
    var origSrc = originalUrl(fullImgEl.getAttribute("data-src") || "");
    var altText = fullImgEl.getAttribute("alt") || "";

    // 淡出当前图
    s.img.style.transition = "opacity 150ms ease-out";
    s.img.style.opacity = "0";

    // 150ms 后换图淡入
    var swap = function (): void {
      if (!state) return; // 切换中可能已关闭
      var fade = function (): void {
        if (!state) return;
        s.img.style.transition = "opacity 150ms ease-out";
        s.img.style.opacity = "1";
      };
      if (s.img.complete && s.img.naturalWidth) {
        // 缓存命中，直接淡入
        s.img.src = origSrc;
        fade();
      } else {
        s.img.addEventListener("load", fade, { once: true });
        s.img.src = origSrc;
      }
      s.caption.textContent = altText;
      s.caption.style.display = altText ? "" : "none";
      s.counter.textContent = newIndex + 1 + " / " + s.gallery.length;
      // 更新 originNode 为新图，使后续关闭/滚动关闭飞回新图位置
      s.originNode = newNode;
      s.index = newIndex;
      s.openScrollY = window.scrollY; // 重置滚动关闭基线
    };
    setTimeout(swap, 150);
  }

  // ============ 交互绑定 ============

  function bindInteractions(): void {
    var s = state;
    if (!s) return;

    // 点背景关闭（点图片本身不关，因箭头在图上、避免误关）
    s.overlay.addEventListener("click", function (ev: MouseEvent): void {
      if (state && ev.target === state.overlay) closeLightbox(false);
    });

    // 滚动驱动关闭：任何 scroll 都触发，用 scrollY 偏移算进度。
    // 关键：逐帧读 originNode 实时 rect，文章滚多少图就回多少。
    s.scrollHandler = function (): void {
      if (!state) return;
      var st = state;
      if (st.closing) return;
      var dy = Math.abs(window.scrollY - st.openScrollY);
      if (st.reduced) {
        // reduced-motion：立即关
        closeLightbox(true);
        return;
      }
      var target = st.target;
      var baseW = st.baseW || (target ? target.w : 1);
      var baseH = st.baseH || (target ? target.h : 1);
      var originRect = rectOf(st.originNode);
      // 在 originRect 与居中 target 之间按 progress 线性插值
      var progress = Math.min(dy / 120, 1);
      if (!target) return; // 无 target 时不插值（忠实原 JS 的兜底语义）
      var cur: Rect = {
        x: target.x + (originRect.x - target.x) * progress,
        y: target.y + (originRect.y - target.y) * progress,
        w: target.w + (originRect.w - target.w) * progress,
        h: target.h + (originRect.h - target.h) * progress,
      };
      st.img.style.transition = "none";
      st.img.style.transform = transformFor(cur, baseW, baseH);
      st.img.style.opacity = String(1 - progress);
      st.overlay.style.opacity = String(1 - progress);
      if (progress >= 1) {
        // 已飞回原位：文章停在当前滚动位置，移除灯箱
        st.closing = true;
        cleanupInteractions();
        removeOverlay();
      }
    };
    window.addEventListener("scroll", s.scrollHandler, { passive: true });

    // 键盘：Esc 关；图集模式 ←→ 切换
    s.keyHandler = function (ev: KeyboardEvent): void {
      if (!state) return;
      if (ev.key === "Escape") {
        closeLightbox(false);
      } else if (!state.isSingle && state.gallery.length > 1) {
        if (ev.key === "ArrowLeft") {
          ev.preventDefault();
          gotoIndex((state.index ?? 0) - 1);
        } else if (ev.key === "ArrowRight") {
          ev.preventDefault();
          gotoIndex((state.index ?? 0) + 1);
        }
      }
    };
    document.addEventListener("keydown", s.keyHandler);

    // 图集导航箭头点击（stopPropagation 防止冒泡到 overlay 触发关闭）
    if (s.prevBtn) {
      s.prevBtn.addEventListener("click", function (ev: MouseEvent): void {
        ev.stopPropagation();
        if (state) gotoIndex((state.index ?? 0) - 1);
      });
    }
    if (s.nextBtn) {
      s.nextBtn.addEventListener("click", function (ev: MouseEvent): void {
        ev.stopPropagation();
        if (state) gotoIndex((state.index ?? 0) + 1);
      });
    }
  }

  function cleanupInteractions(): void {
    if (!state) return;
    if (state.scrollHandler) {
      window.removeEventListener("scroll", state.scrollHandler);
      state.scrollHandler = null;
    }
    if (state.keyHandler) {
      document.removeEventListener("keydown", state.keyHandler);
      state.keyHandler = null;
    }
  }

  // ============ 初始化入口 ============

  window.__initLightbox = function (selectors: string | string[]): void {
    // selectors 可以是字符串、字符串数组
    var sels = Array.isArray(selectors) ? selectors : [selectors];
    var roots: Element[] = [];
    for (var i = 0; i < sels.length; i++) {
      var found = document.querySelectorAll(sels[i]);
      for (var j = 0; j < found.length; j++) roots.push(found[j]);
    }

    // 先对所有图片做懒加载（图集与单张都做）
    var collected = collectImages(roots);
    var everyone = collected.gallery.concat(collected.singles);
    for (var k = 0; k < everyone.length; k++) {
      initLazyLoad(everyone[k]);
    }

    // 正文图：带 index
    var gallery = collected.gallery;
    for (var g = 0; g < gallery.length; g++) {
      (function (node: HTMLElement, idx: number): void {
        node.addEventListener("click", function (e: MouseEvent): void {
          e.preventDefault();
          openLightbox(node, gallery, idx);
        });
      })(gallery[g], g);
    }
    // 单张图（封面）：index = null，gallery 传空数组表示单张
    for (var si = 0; si < collected.singles.length; si++) {
      (function (node: HTMLElement): void {
        node.addEventListener("click", function (e: MouseEvent): void {
          e.preventDefault();
          openLightbox(node, [], null);
        });
      })(collected.singles[si]);
    }
  };

  // ============ 自启动 ============
  // 方案 iii：双保险契约，无需轮询。
  // 1) Rust 内联 eval 先跑（常态）：设 __lightboxSelectors，此时 __initLightbox 可能未定义 → 只设配置；
  //    lightbox.js 后加载完 → 读到配置 → 这里自启动。
  // 2) lightbox.js 先加载完：__initLightbox 就绪但无配置 → 不自启动；
  //    Rust eval 后跑 → 设配置 + 兜底 if(__initLightbox) 显式调用 → 初始化。
  if (Array.isArray(window.__lightboxSelectors)) {
    window.__initLightbox(window.__lightboxSelectors);
  }
})();
