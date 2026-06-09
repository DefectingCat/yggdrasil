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
    var images = root.querySelectorAll("img");
    for (var i = 0; i < images.length; i++) {
      var img = images[i];
      if (img.getAttribute("data-zoom-enabled")) continue;
      var src = img.getAttribute("src") || img.src || "";
      if (src.indexOf("data:") === 0) continue;

      img.setAttribute("data-zoom-enabled", "true");

      var originalSrc = img.src;
      var sep = originalSrc.indexOf("?") !== -1 ? "&" : "?";
      img.src = originalSrc + sep + "w=800";
      img.classList.add("md-content-img-zoomable");

      (function (origSrc, altText) {
        img.addEventListener("click", function (e) {
          e.preventDefault();
          var overlay = document.createElement("div");
          overlay.className = "md-image-lightbox-overlay";

          var container = document.createElement("div");
          container.className = "md-image-lightbox-content";

          var fullImg = document.createElement("img");
          fullImg.src = origSrc;
          fullImg.alt = altText;

          var closeBtn = document.createElement("button");
          closeBtn.className = "md-image-lightbox-close";
          closeBtn.textContent = "\u2715";

          container.appendChild(fullImg);
          container.appendChild(closeBtn);
          overlay.appendChild(container);
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
          container.addEventListener("click", function (ev) {
            ev.stopPropagation();
          });
          closeBtn.addEventListener("click", function () {
            cleanup(overlay, onKey);
          });
          document.addEventListener("keydown", onKey);
        });
      })(originalSrc, img.alt || "");
    }
  }

  window.__initPostContent = function (selector) {
    var root = document.querySelector(selector);
    if (!root) return;
    initCopyButtons(root);
    initImageZoom(root);
  };
})();
