(function () {
  "use strict";

  // ============ 工具函数 ============

  function prefersReducedMotion() {
    return (
      window.matchMedia &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    );
  }

  // 读取元素当前在视口里的 rect（用于飞行起点/终点）。
  function rectOf(el) {
    return el.getBoundingClientRect();
  }

  // 计算图片在视口居中、contain 适配后的目标 rect。
  // naturalW/H: 图片真实像素尺寸；vw/vh: 视口尺寸。
  function fitCentered(naturalW, naturalH, vw, vh) {
    var maxW = vw * 0.92;
    var maxH = vh * 0.88;
    var scale = Math.min(maxW / naturalW, maxH / naturalH, 1);
    var w = naturalW * scale;
    var h = naturalH * scale;
    return {
      x: (vw - w) / 2,
      y: (vh - h) / 2,
      w: w,
      h: h,
    };
  }

  // 把目标 rect 转成 transform 字符串（基于 naturalW/H 做缩放）。
  // transform-origin 为 top left（见 CSS），translate 到 rect 左上角后 scale。
  function transformFor(rect, naturalW, naturalH) {
    var sx = rect.w / naturalW;
    var sy = rect.h / naturalH;
    return (
      "translate(" +
      rect.x +
      "px," +
      rect.y +
      "px) scale(" +
      sx +
      "," +
      sy +
      ")"
    );
  }

  // 原图 URL = data-src 去 query。data-src 形如 "/uploads/x.webp?w=800"。
  function originalUrl(dataSrc) {
    return (dataSrc || "").split("?")[0];
  }

  // ============ 懒加载 ============

  // 为单个 .blur-img 容器初始化高清图懒加载。
  // IO 进入视口后把 data-src 写入 src，加载完成加 is-loaded 触发 CSS 淡入。
  function initLazyLoad(container) {
    var fullImg = container.querySelector(".blur-img-full");
    if (!fullImg) return;
    if (container.getAttribute("data-blur-init")) return;
    container.setAttribute("data-blur-init", "true");

    var fullSrc = fullImg.getAttribute("data-src");
    if (!fullSrc) return;

    fullImg.addEventListener("load", function () {
      this.classList.add("is-loaded");
    });

    if ("IntersectionObserver" in window) {
      var io = new IntersectionObserver(
        function (entries) {
          entries.forEach(function (entry) {
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
  // gallery: 正文图（组成图集）；singles: 带 data-single 的单张图（如封面）。
  function collectImages(roots) {
    var gallery = [];
    var singles = [];
    for (var i = 0; i < roots.length; i++) {
      var nodes = roots[i].querySelectorAll(".blur-img");
      for (var j = 0; j < nodes.length; j++) {
        var n = nodes[j];
        if (n.getAttribute("data-single")) {
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
  var state = null;

  function openLightbox(originNode, gallery, index) {
    if (state) closeLightbox(true);

    var fullImg = originNode.querySelector(".blur-img-full");
    if (!fullImg) return;
    var dataSrc = fullImg.getAttribute("data-src") || "";
    var origSrc = originalUrl(dataSrc);
    var altText = fullImg.getAttribute("alt") || "";
    var isSingle =
      originNode.getAttribute("data-single") === "true" || gallery.length === 0;

    var vw = window.innerWidth;
    var vh = window.innerHeight;

    // 建 DOM
    var overlay = document.createElement("div");
    overlay.className = "lightbox-overlay";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-modal", "true");
    overlay.setAttribute("aria-label", "图片预览");
    overlay.setAttribute("tabindex", "-1");

    var img = document.createElement("img");
    img.className = "lightbox-img";
    img.setAttribute("alt", altText);

    var caption = document.createElement("figcaption");
    caption.className = "lightbox-caption";
    caption.textContent = altText;
    if (!altText) caption.style.display = "none";

    var counter = document.createElement("div");
    counter.className = "lightbox-counter";
    if (isSingle || gallery.length === 0) {
      counter.style.display = "none";
    } else {
      counter.textContent = index + 1 + " / " + gallery.length;
    }

    // 图集模式（>1 张）才加左右导航箭头；单张不显示。
    var prevBtn = null;
    var nextBtn = null;
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
    var start = function () {
      if (!state) return; // 加载前可能已被关闭
      var naturalW = img.naturalWidth || img.clientWidth || 1;
      var naturalH = img.naturalHeight || img.clientHeight || 1;
      var originRect = rectOf(originNode);

      // reduced-motion：直接淡入居中
      if (state.reduced) {
        img.style.opacity = "0";
        var target = fitCentered(naturalW, naturalH, vw, vh);
        img.style.transform = transformFor(target, naturalW, naturalH);
        img.style.left = "0";
        img.style.top = "0";
        overlay.style.opacity = "0";
        // 下一帧淡入
        requestAnimationFrame(function () {
          if (!state) return;
          overlay.style.transition = "opacity 200ms ease-out";
          img.style.transition = "opacity 200ms ease-out";
          overlay.style.opacity = "1";
          img.style.opacity = "1";
        });
        return;
      }

      // 首帧：放到 originRect 位置与尺寸，透明
      img.style.transition = "none";
      img.style.left = "0";
      img.style.top = "0";
      img.style.transform = transformFor(originRect, naturalW, naturalH);
      img.style.opacity = "0";
      overlay.style.opacity = "0";

      // 下一帧：飞到居中
      requestAnimationFrame(function () {
        if (!state) return;
        var tgt = fitCentered(naturalW, naturalH, vw, vh);
        img.style.transition =
          "transform 250ms ease-out, opacity 250ms ease-out";
        overlay.style.transition = "opacity 250ms ease-out";
        img.style.transform = transformFor(tgt, naturalW, naturalH);
        img.style.opacity = "1";
        overlay.style.opacity = "1";
      });
    };

    if (img.complete && img.naturalWidth) {
      start();
    } else {
      img.addEventListener("load", start, { once: true });
    }
    img.src = origSrc;
  }

  function closeLightbox(immediate) {
    if (!state || state.closing) return;
    state.closing = true;
    cleanupInteractions();

    var s = state;

    var naturalW = s.img.naturalWidth || s.img.clientWidth || 1;
    var naturalH = s.img.naturalHeight || s.img.clientHeight || 1;
    var originRect = rectOf(s.originNode); // 实时读，处理期间滚动过的情况

    if (s.reduced || immediate) {
      removeOverlay();
      return;
    }

    // 飞回 originRect
    s.img.style.transition =
      "transform 250ms ease-out, opacity 250ms ease-out";
    s.overlay.style.transition = "opacity 250ms ease-out";
    s.img.style.transform = transformFor(originRect, naturalW, naturalH);
    s.img.style.opacity = "0";
    s.overlay.style.opacity = "0";

    var done = function () {
      removeOverlay();
    };
    // 250ms 兜底，避免 transitionend 不触发
    setTimeout(done, 280);
    s.img.addEventListener("transitionend", done, { once: true });
  }

  function removeOverlay() {
    if (!state) return;
    var prev = state.originNode;
    if (state.overlay && state.overlay.parentNode) {
      state.overlay.parentNode.removeChild(state.overlay);
    }
    state = null;
    // 焦点归还：.blur-img 是 span 不可聚焦，让其内部 full img 获得焦点
    if (prev) {
      var f = prev.querySelector(".blur-img-full");
      if (f) {
        f.setAttribute("tabindex", "-1");
        f.focus();
      }
    }
  }

  // ============ 图集切换 ============

  // 图集切换：淡入淡出，不飞行。newIndex 循环（首尾衔接）。
  function gotoIndex(newIndex) {
    if (!state || state.isSingle) return;
    var s = state;
    if (!s.gallery || s.gallery.length === 0) return;
    if (newIndex < 0) newIndex = s.gallery.length - 1;
    if (newIndex >= s.gallery.length) newIndex = 0;
    if (newIndex === s.index) return;

    var newNode = s.gallery[newIndex];
    var fullImg = newNode.querySelector(".blur-img-full");
    if (!fullImg) return;
    var origSrc = originalUrl(fullImg.getAttribute("data-src") || "");
    var altText = fullImg.getAttribute("alt") || "";

    // 淡出当前图
    s.img.style.transition = "opacity 150ms ease-out";
    s.img.style.opacity = "0";

    // 150ms 后换图淡入
    var swap = function () {
      if (!state) return; // 切换中可能已关闭
      var fade = function () {
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

  function bindInteractions() {
    var s = state;
    if (!s) return;

    // 点背景关闭（点图片本身不关，因箭头在图上、避免误关）
    s.overlay.addEventListener("click", function (ev) {
      if (state && ev.target === state.overlay) closeLightbox(false);
    });

    // 滚动驱动关闭：任何 scroll 都触发，用 scrollY 偏移算进度。
    // 关键：逐帧读 originNode 实时 rect，文章滚多少图就回多少。
    s.scrollHandler = function () {
      if (!state) return;
      var st = state;
      if (st.closing) return;
      var dy = Math.abs(window.scrollY - st.openScrollY);
      if (st.reduced) {
        // reduced-motion：立即关
        closeLightbox(true);
        return;
      }
      var progress = Math.min(dy / 120, 1);
      var naturalW = st.img.naturalWidth || st.img.clientWidth || 1;
      var naturalH = st.img.naturalHeight || st.img.clientHeight || 1;
      var originRect = rectOf(st.originNode);
      var target = fitCentered(
        naturalW,
        naturalH,
        window.innerWidth,
        window.innerHeight
      );
      // 在 originRect 与居中 target 之间按 progress 线性插值
      var cur = {
        x: target.x + (originRect.x - target.x) * progress,
        y: target.y + (originRect.y - target.y) * progress,
        w: target.w + (originRect.w - target.w) * progress,
        h: target.h + (originRect.h - target.h) * progress,
      };
      st.img.style.transition = "none";
      st.img.style.transform = transformFor(cur, naturalW, naturalH);
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
    s.keyHandler = function (ev) {
      if (!state) return;
      if (ev.key === "Escape") {
        closeLightbox(false);
      } else if (!state.isSingle && state.gallery.length > 1) {
        if (ev.key === "ArrowLeft") {
          ev.preventDefault();
          gotoIndex(state.index - 1);
        } else if (ev.key === "ArrowRight") {
          ev.preventDefault();
          gotoIndex(state.index + 1);
        }
      }
    };
    document.addEventListener("keydown", s.keyHandler);

    // 图集导航箭头点击（stopPropagation 防止冒泡到 overlay 触发关闭）
    if (s.prevBtn) {
      s.prevBtn.addEventListener("click", function (ev) {
        ev.stopPropagation();
        if (state) gotoIndex(state.index - 1);
      });
    }
    if (s.nextBtn) {
      s.nextBtn.addEventListener("click", function (ev) {
        ev.stopPropagation();
        if (state) gotoIndex(state.index + 1);
      });
    }
  }

  function cleanupInteractions() {
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

  window.__initLightbox = function (selectors) {
    // selectors 可以是字符串、字符串数组
    var sels = Array.isArray(selectors) ? selectors : [selectors];
    var roots = [];
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
      (function (node, idx) {
        node.addEventListener("click", function (e) {
          e.preventDefault();
          openLightbox(node, gallery, idx);
        });
      })(gallery[g], g);
    }
    // 单张图（封面）：index = null，gallery 传空数组表示单张
    for (var si = 0; si < collected.singles.length; si++) {
      (function (node) {
        node.addEventListener("click", function (e) {
          e.preventDefault();
          openLightbox(node, [], null);
        });
      })(collected.singles[si]);
    }
  };
})();
