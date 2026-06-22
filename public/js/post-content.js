(function () {
  "use strict";

  function initCopyButtons(root) {
    var blocks = root.querySelectorAll("pre > code");
    for (var i = 0; i < blocks.length; i++) {
      var code = blocks[i];
      var pre = code.parentElement;
      if (!pre) continue;
      if (pre.querySelector(".copy-code")) continue;

      var btn = document.createElement("button");
      btn.className = "copy-code";
      btn.textContent = "copy";
      btn.setAttribute("aria-label", "Copy code");

      (function (codeText) {
        btn.addEventListener("click", function () {
          var self = this;
          if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(codeText);
          } else {
            var ta = document.createElement("textarea");
            ta.value = codeText;
            ta.style.position = "fixed";
            ta.style.opacity = "0";
            document.body.appendChild(ta);
            ta.select();
            document.execCommand("copy");
            document.body.removeChild(ta);
          }
          self.textContent = "copied!";
          setTimeout(function () {
            self.textContent = "copy";
          }, 2000);
        });
      })(code.textContent || "");

      pre.appendChild(btn);
    }
  }

  function closeLightbox(overlay) {
    if (overlay && overlay.parentNode) {
      overlay.parentNode.removeChild(overlay);
      document.body.style.overflow = "";
    }
  }

  function initImageZoom(root) {
    var containers = root.querySelectorAll(".blur-img");
    for (var i = 0; i < containers.length; i++) {
      var container = containers[i];
      if (container.getAttribute("data-blur-init")) continue;
      container.setAttribute("data-blur-init", "true");

      var fullImg = container.querySelector(".blur-img-full");
      if (!fullImg) continue;
      var fullSrc = fullImg.getAttribute("data-src");
      if (!fullSrc) continue;

      // 加载高清图：onload 后加 is-loaded 触发 CSS opacity 淡入
      fullImg.addEventListener("load", function () {
        this.classList.add("is-loaded");
      });

      // 懒加载：进入视口才设 src
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
        // 不支持 IO：直接加载
        fullImg.src = fullSrc;
      }

      // 灯箱：点击用高清图 URL 放大
      (function (src, altText) {
        container.addEventListener("click", function (e) {
          e.preventDefault();
          var overlay = document.createElement("div");
          overlay.className = "md-image-lightbox-overlay";

          var containerEl = document.createElement("div");
          containerEl.className = "md-image-lightbox-content";

          var bigImg = document.createElement("img");
          bigImg.src = src;
          bigImg.alt = altText;

          var closeBtn = document.createElement("button");
          closeBtn.className = "md-image-lightbox-close";
          closeBtn.textContent = "\u2715";

          containerEl.appendChild(bigImg);
          containerEl.appendChild(closeBtn);
          overlay.appendChild(containerEl);
          document.body.appendChild(overlay);
          document.body.style.overflow = "hidden";

          var onKey = function (ev) {
            if (ev.key === "Escape") {
              cleanup(overlay, onKey);
            }
          };
          var cleanup = function (ol, kh) {
            closeLightbox(ol);
            document.removeEventListener("keydown", kh);
          };
          overlay.addEventListener("click", function () {
            cleanup(overlay, onKey);
          });
          containerEl.addEventListener("click", function (ev) {
            ev.stopPropagation();
          });
          closeBtn.addEventListener("click", function () {
            cleanup(overlay, onKey);
          });
          document.addEventListener("keydown", onKey);
        });
      })(fullSrc, fullImg.getAttribute("alt") || "");
    }
  }

  window.__initPostContent = function (selector) {
    var root = document.querySelector(selector);
    if (!root) return;
    initCopyButtons(root);
    initImageZoom(root);
  };
})();
