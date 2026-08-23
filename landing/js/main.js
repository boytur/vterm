(() => {
  "use strict";

  document.getElementById("year").textContent = new Date().getFullYear();
  loadChangelog();

  const themes = [
    { name: "Light", bg: "#f7f8fa", accent: "#0969da" },
    { name: "Midnight", bg: "#141827", accent: "#8ea7ff" },
    { name: "Ocean", bg: "#10202a", accent: "#4fd1c5" },
    { name: "Forest", bg: "#17201a", accent: "#8fcf72" },
    { name: "Rose", bg: "#21171d", accent: "#f08cae" },
    { name: "Paper", bg: "#fbf8f2", accent: "#b45f28" },
    { name: "Lavender", bg: "#f7f4ff", accent: "#7957c8" },
    { name: "Sand", bg: "#f8f3e8", accent: "#b86b3e" },
    { name: "High Contrast", bg: "#000000", accent: "#00e5ff" },
    { name: "One Light", bg: "#fafafa", accent: "#4078f2" },
    { name: "VS Code Light+", bg: "#ffffff", accent: "#0066bf" },
    { name: "VS Code Quiet Light", bg: "#f5f5f5", accent: "#2f6f9f" },
    { name: "Solarized Light", bg: "#fdf6e3", accent: "#268bd2" },
    { name: "Ubuntu", bg: "#300a24", accent: "#e95420" },
    { name: "Zed Dark", bg: "#1e1e1e", accent: "#007fd4" },
    { name: "VS Code Dark+", bg: "#1e1e1e", accent: "#007acc" },
    { name: "VS Code Abyss", bg: "#000c18", accent: "#75beff" },
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
  themes.forEach((t) => {
    const card = document.createElement("button");
    card.className = "theme-card";
    card.type = "button";
    card.setAttribute("aria-pressed", "false");
    card.innerHTML = `
      <div class="theme-swatches" aria-hidden="true">
        <span class="theme-swatch" style="background:${t.bg}"></span>
        <span class="theme-swatch" style="background:${t.accent}"></span>
      </div>
      <span class="theme-name">${t.name}</span>`;
    card.addEventListener("click", () => applyTheme(t, card));
    grid.appendChild(card);
  });

  function applyTheme(t, card) {
    document.documentElement.style.setProperty("--accent", t.accent);
    document.documentElement.style.setProperty("--accent-bright", t.accent);
    document.querySelectorAll(".theme-card").forEach((c) => {
      c.classList.remove("active");
      c.setAttribute("aria-pressed", "false");
    });
    card.classList.add("active");
    card.setAttribute("aria-pressed", "true");
  }

  async function loadChangelog() {
    const version = document.getElementById("release-version");
    const title = document.getElementById("release-title");
    const items = document.getElementById("release-items");
    const history = document.getElementById("release-history-list");

    try {
      const releases = parseChangelog(await fetchChangelog());
      const unreleased = releases.find((release) => release.version === "Unreleased" && release.items.length);
      const latest = releases.find((release) => release.version !== "Unreleased");
      const featured = unreleased || latest;
      if (!featured) throw new Error("no release notes");

      version.textContent = unreleased ? "Unreleased" : `v${featured.version}`;
      title.textContent = unreleased ? "Coming next" : `v${featured.version}`;
      renderItems(items, featured.items);
      renderHistory(history, releases.filter((release) => release.version !== "Unreleased"));
    } catch {
      version.textContent = "Updates";
      title.textContent = "Release notes unavailable";
      const message = document.createElement("li");
      message.textContent = "See GitHub for the latest release history.";
      items.replaceChildren(message);
    }
  }

  async function fetchChangelog() {
    const sources = [
      "./changelog.md",
      "https://raw.githubusercontent.com/boytur/vterm/master/CHANGELOG.md",
    ];
    for (const source of sources) {
      try {
        const response = await fetch(source, { cache: "no-store" });
        if (response.ok) return response.text();
      } catch {}
    }
    throw new Error("changelog unavailable");
  }

  function parseChangelog(markdown) {
    const releases = [];
    let current;
    markdown.split(/\r?\n/).forEach((line) => {
      const heading = line.match(/^## \[([^\]]+)\](?:\s+-\s+(.+))?$/);
      if (heading) {
        current = { version: heading[1], date: heading[2] || "", items: [] };
        releases.push(current);
      } else if (current && /^\s*-\s+/.test(line)) {
        current.items.push(line.replace(/^\s*-\s+/, ""));
      }
    });
    return releases;
  }

  function renderItems(list, entries) {
    list.replaceChildren(...entries.map((entry) => {
      const item = document.createElement("li");
      item.textContent = entry;
      return item;
    }));
  }

  function renderHistory(container, releases) {
    container.replaceChildren(...releases.map((release) => {
      const section = document.createElement("section");
      const heading = document.createElement("h3");
      heading.className = "text-sm font-semibold text-white";
      heading.textContent = `v${release.version}${release.date ? ` · ${release.date}` : ""}`;
      const list = document.createElement("ul");
      list.className = "mt-2 space-y-2 text-sm text-gray-400 list-disc list-inside";
      renderItems(list, release.items);
      section.append(heading, list);
      return section;
    }));
  }

  document.querySelectorAll("[data-copy-target]").forEach((button) => {
    button.addEventListener("click", async () => {
      const target = document.getElementById(button.dataset.copyTarget);
      if (!target) return;

      try {
        await copyText(target.textContent.trim());
        button.textContent = "Copied";
        setTimeout(() => {
          button.textContent = "Copy";
        }, 1600);
      } catch {
        button.textContent = "Copy failed";
        setTimeout(() => {
          button.textContent = "Copy";
        }, 2200);
      }
    });
  });

  async function copyText(text) {
    try {
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(text);
        return;
      }
    } catch {}

    const input = document.createElement("textarea");
    input.value = text;
    input.style.position = "fixed";
    input.style.opacity = "0";
    document.body.appendChild(input);
    input.select();
    const copied = document.execCommand("copy");
    input.remove();
    if (!copied) throw new Error("copy failed");
  }

  const demoLines = [
    { cls: "prompt", text: "$ " },
    { cls: "cmd", text: "cd ~/projects/vterm\n" },
    { cls: "cmd", text: "git status --short\n" },
    { cls: "ok", text: "  working tree clean\n" },
    { cls: "out", text: "  vterm ready · 27 themes · sessions restored\n" },
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
