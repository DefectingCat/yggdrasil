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

  window.__initPostContent = function (selector) {
    var root = document.querySelector(selector);
    if (!root) return;
    initCopyButtons(root);
  };
})();
