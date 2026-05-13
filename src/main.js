const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const searchInput = document.getElementById("search");
const resultsContainer = document.getElementById("results");
let selectedIndex = 0;
let currentResults = [];
let debounceTimer = null;

// Focus search input when window becomes visible
listen("trigger-activated", () => {
  resetState();
});

// Also reset on window focus
getCurrentWindow().listen("tauri://focus", () => {
  resetState();
});

// Dismiss when clicking away (window loses focus)
getCurrentWindow().listen("tauri://blur", () => {
  invoke("dismiss");
});

function resetState() {
  searchInput.value = "";
  selectedIndex = 0;
  searchInput.focus();
  doSearch("");
}

// Search on input with debounce
searchInput.addEventListener("input", () => {
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    doSearch(searchInput.value.trim());
  }, 50);
});

// Keyboard navigation
searchInput.addEventListener("keydown", (e) => {
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
      renderResults();
      scrollToSelected();
      break;
    case "ArrowUp":
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      renderResults();
      scrollToSelected();
      break;
    case "Enter":
      e.preventDefault();
      if (currentResults[selectedIndex]) {
        selectEmoji(currentResults[selectedIndex].emoji);
      }
      break;
    case "Escape":
      e.preventDefault();
      invoke("dismiss");
      break;
  }
});

async function doSearch(query) {
  try {
    currentResults = await invoke("search_emojis", { query });
    selectedIndex = 0;
    renderResults();
    resultsContainer.scrollTop = 0;
  } catch (err) {
    console.error("Search failed:", err);
  }
}

function renderResults() {
  resultsContainer.innerHTML = currentResults
    .map(
      (r, i) => `
    <div class="emoji-row ${i === selectedIndex ? "selected" : ""}" data-index="${i}">
      <span class="emoji">${r.emoji}</span>
      <span class="name">${escapeHtml(r.name)}</span>
    </div>
  `
    )
    .join("");

  // Click handler for each row
  resultsContainer.querySelectorAll(".emoji-row").forEach((row) => {
    row.addEventListener("click", () => {
      const idx = parseInt(row.dataset.index);
      selectEmoji(currentResults[idx].emoji);
    });
  });
}

function scrollToSelected() {
  const selected = resultsContainer.querySelector(".selected");
  if (selected) {
    selected.scrollIntoView({ block: "nearest" });
  }
}

async function selectEmoji(emoji) {
  try {
    await invoke("select_emoji", { emoji });
  } catch (err) {
    console.error("Failed to paste emoji:", err);
  }
}

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

// Initial search to populate with default emojis
doSearch("");
