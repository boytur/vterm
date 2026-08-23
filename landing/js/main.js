(() => {
  "use strict";

  document.getElementById("year").textContent = new Date().getFullYear();

  const themes = [
    { name: "Ubuntu", bg: "#300a24", accent: "#e95420" },
    { name: "Zed Dark", bg: "#1e1e1e", accent: "#007fd4" },
    { name: "Dracula", bg: "#282a36", accent: "#bd93f9" },
    { name: "Nord", bg: "#2e3440", accent: "#88c0d0" },
    { name: "Gruvbox", bg: "#282828", accent: "#fe8019" },
    { name: "One Dark", bg: "#282c34", accent: "#61afef" },
    { name: "Tokyo Night", bg: "#1a1b26", accent: "#7aa2f7" },
    { name: "Catppuccin", bg: "#1e1e2e", accent: "#cba6f7" },
    { name: "Monokai", bg: "#272822", accent: "#f92672" },
    { name: "GitHub Dark", bg: "#0d1117", accent: "#58a6ff" },
    { name: "Solarized", bg: "#002b36", accent: "#268bd2" },
    { name: "Ayu Dark", bg: "#0f1419", accent: "#ffb454" },
  ];

  const grid = document.getElementById("theme-grid");
  themes.forEach((t, i) => {
    const card = document.createElement("button");
    card.className =
      "theme-card text-left p-4 rounded-xl border border-edge bg-panel";
    card.innerHTML = `
      <div class="flex items-center gap-2 mb-3">
        <span class="w-4 h-4 rounded-full" style="background:${t.bg};border:1px solid #30363d"></span>
        <span class="w-4 h-4 rounded-full" style="background:${t.accent}"></span>
      </div>
      <div class="text-sm text-white font-semibold">${t.name}</div>`;
    card.addEventListener("click", () => applyTheme(t, card));
    grid.appendChild(card);
  });

  function applyTheme(t, card) {
    document.documentElement.style.setProperty("--tw-prose-links", t.accent);
    document.querySelectorAll(".theme-card").forEach((c) => c.classList.remove("active"));
    card.classList.add("active");
    document.querySelectorAll(".text-accent").forEach((el) => {
      el.style.color = t.accent;
    });
    document.querySelectorAll(".bg-accent").forEach((el) => {
      el.style.backgroundColor = t.accent;
    });
    document.querySelectorAll(".border-accent").forEach((el) => {
      el.style.borderColor = t.accent;
    });
  }

  const demoLines = [
    { cls: "prompt", text: "$ " },
    { cls: "cmd", text: "cargo run --release\n" },
    { cls: "out", text: "   Compiling vterm v0.1.0\n" },
    { cls: "ok", text: "    Finished release [optimized]\n" },
    { cls: "out", text: "  vterm ready · 12 themes loaded\n" },
  ];

  const demo = document.getElementById("terminal-demo");
  let li = 0,
    ci = 0;
  function typeDemo() {
    if (li >= demoLines.length) return;
    const line = demoLines[li];
    if (ci === 0) {
      const span = document.createElement("span");
      span.className = line.cls;
      demo.appendChild(span);
    }
    const span = demo.lastChild;
    span.textContent += line.text[ci];
    ci++;
    if (ci >= line.text.length) {
      li++;
      ci = 0;
    }
    setTimeout(typeDemo, 22);
  }
  typeDemo();
})();
