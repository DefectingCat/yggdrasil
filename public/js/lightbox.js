(function () {
  "use strict";

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

  window.__initLightbox = function (selectors) {
    // selectors 可以是字符串、字符串数组
    var sels = Array.isArray(selectors) ? selectors : [selectors];
    var roots = [];
    for (var i = 0; i < sels.length; i++) {
      var found = document.querySelectorAll(sels[i]);
      for (var j = 0; j < found.length; j++) roots.push(found[j]);
    }

    // 先对所有图片做懒加载（图集与单张都做）
    var all = collectImages(roots);
    var everyone = all.gallery.concat(all.singles);
    for (var k = 0; k < everyone.length; k++) {
      initLazyLoad(everyone[k]);
    }

    // click 绑定在 Task 2 完成
  };
})();
