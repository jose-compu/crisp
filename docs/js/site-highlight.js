/**
 * Wrap bare <pre>/<pre class="pipe"> contents in <code class="language-*">
 * when missing, then run Prism.
 *
 * Heuristic: Crisp if it looks like .crp; bash if it looks like shell; else plain.
 */
(function () {
  function looksLikeCrisp(text) {
    return (
      /(?:^|\n)\s*(?:pub\s+)?(?:main|type|test|use|extern|trait|impl|shape|enum)\b/.test(
        text
      ) ||
      /(?:^|\n)\s*impl\s+\w+\s+for\s+\w+/.test(text) ||
      /:=/.test(text) ||
      /\+\+/.test(text) ||
      /(?:^|\n)\s*--/.test(text) ||
      /\bthen\b/.test(text) ||
      /\bcatch\b/.test(text) ||
      /\bthrow\b/.test(text) ||
      /\bmatch\b/.test(text) ||
      /(?:^|\n)\s*\w[\w']*\([^)]*\)\s*=/.test(text)
    );
  }

  function looksLikeShell(text) {
    return (
      /(?:^|\n)\s*(?:git|cargo|crpc|export|cd|#)\b/.test(text) ||
      /\$[A-Z_]/.test(text)
    );
  }

  function ensureCode(pre, lang) {
    if (pre.querySelector("code")) return;
    var code = document.createElement("code");
    code.className = "language-" + lang;
    code.textContent = pre.textContent;
    pre.textContent = "";
    pre.appendChild(code);
    if (!pre.classList.contains("language-" + lang)) {
      pre.classList.add("language-" + lang);
    }
  }

  document.querySelectorAll("pre").forEach(function (pre) {
    if (pre.querySelector("code[class*='language-']")) {
      var c = pre.querySelector("code");
      var m = c.className.match(/language-(\w+)/);
      if (m) pre.classList.add("language-" + m[1]);
      return;
    }
    var text = pre.textContent || "";
    if (!text.trim()) return;
    if (looksLikeCrisp(text)) ensureCode(pre, "crisp");
    else if (looksLikeShell(text)) ensureCode(pre, "bash");
    else ensureCode(pre, "none");
  });

  if (window.Prism) {
    Prism.highlightAll();
  }
})();
