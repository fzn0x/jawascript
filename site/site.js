// The documentation site's small behaviours: the theme switch, copy buttons
// on code, the vocabulary filter and the "on this page" highlight. Every page
// reads fine without this file.

(function () {
  "use strict";

  var root = document.documentElement;

  // ---- Theme ----------------------------------------------------------

  var toggle = document.querySelector(".theme-toggle");
  var themeColor = document.querySelector('meta[name="theme-color"]');

  function applyTheme(theme) {
    root.dataset.theme = theme;
    var next = theme === "dark" ? "light" : "dark";
    toggle.setAttribute("aria-label", "Switch to the " + next + " theme");
    toggle.title = "Switch to the " + next + " theme";
    if (themeColor) themeColor.content = theme === "dark" ? "#1c1a17" : "#f7f2e8";
  }

  if (toggle) {
    toggle.hidden = false;
    applyTheme(root.dataset.theme === "light" ? "light" : "dark");
    toggle.addEventListener("click", function () {
      var theme = root.dataset.theme === "dark" ? "light" : "dark";
      applyTheme(theme);
      try {
        localStorage.setItem("jawascript-theme", theme);
      } catch (e) {}
    });
  }

  // ---- Copy buttons ---------------------------------------------------

  if (navigator.clipboard) {
    document.querySelectorAll(".code").forEach(function (block) {
      var label = block.dataset.label;
      if (label === "output") return;
      var button = document.createElement("button");
      button.type = "button";
      button.className = "copy";
      button.textContent = "Copy";
      button.setAttribute("aria-label", "Copy this " + label);
      button.addEventListener("click", function () {
        var text = block.querySelector("code").innerText;
        if (label === "terminal") {
          // Copy the commands, not their output.
          text = text.split("\n").filter(function (line) {
            return line.indexOf("$ ") === 0;
          }).map(function (line) {
            return line.slice(2);
          }).join("\n");
        }
        navigator.clipboard.writeText(text).then(function () {
          button.textContent = "Copied";
          setTimeout(function () {
            button.textContent = "Copy";
          }, 1600);
        }, function () {
          button.textContent = "Press Ctrl+C";
        });
      });
      block.appendChild(button);
    });
  }

  // ---- Vocabulary filter ----------------------------------------------

  var filter = document.querySelector(".filter");
  if (filter) {
    var input = filter.querySelector("input");
    var empty = filter.querySelector(".filter-empty");
    var sections = Array.prototype.slice.call(document.querySelectorAll(".article section"));
    filter.hidden = false;

    var run = function () {
      var query = input.value.trim().toLowerCase();
      var shown = 0;
      sections.forEach(function (section) {
        var rows = section.querySelectorAll("tbody tr");
        if (!rows.length) {
          section.hidden = query !== "";
          return;
        }
        var visible = 0;
        rows.forEach(function (row) {
          var match = !query || row.textContent.toLowerCase().indexOf(query) !== -1;
          row.hidden = !match;
          if (match) visible++;
        });
        section.hidden = visible === 0;
        shown += visible;
      });
      empty.hidden = shown !== 0;
    };

    input.addEventListener("input", run);
    input.addEventListener("keydown", function (event) {
      if (event.key === "Escape") {
        input.value = "";
        run();
      }
    });
    // `/` jumps to the filter, as in many documentation sites.
    document.addEventListener("keydown", function (event) {
      if (event.key === "/" && document.activeElement !== input &&
          !/^(INPUT|TEXTAREA)$/.test(document.activeElement.tagName)) {
        event.preventDefault();
        input.focus();
      }
    });
  }

  // ---- On this page ---------------------------------------------------

  var links = document.querySelectorAll(".toc a");
  if (links.length && "IntersectionObserver" in window) {
    var byId = {};
    links.forEach(function (link) {
      byId[decodeURIComponent(link.hash.slice(1))] = link;
    });
    var headings = Array.prototype.filter.call(document.querySelectorAll(".article h2[id]"), function (h) {
      return byId[h.id];
    });
    var current = null;
    var mark = function (id) {
      if (current) current.removeAttribute("aria-current");
      current = byId[id] || null;
      if (current) current.setAttribute("aria-current", "true");
    };
    var observer = new IntersectionObserver(function () {
      // The active section is the last heading above the top third.
      var line = window.innerHeight / 3;
      var active = headings[0];
      headings.forEach(function (h) {
        if (h.getBoundingClientRect().top < line) active = h;
      });
      if (active) mark(active.id);
    }, { rootMargin: "0px 0px -60% 0px", threshold: [0, 1] });
    headings.forEach(function (h) {
      observer.observe(h);
    });
  }
})();
