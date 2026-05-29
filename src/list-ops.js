// Pure, dependency-free list operations for the torrent list.
// Imported by both the browser app (main.js) and Node tests (list-ops.test.js).

export function matchesSearch(t, query) {
  const q = (query || "").trim().toLowerCase();
  if (!q) return true;
  return (t.name || "").toLowerCase().includes(q);
}

export function filterTorrents(torrents, stateFilter, query) {
  return torrents.filter(t =>
    (stateFilter === "all" || t.state_label === stateFilter) && matchesSearch(t, query)
  );
}

// Numeric/string key extractors for each sort key.
function sortValue(t, key) {
  switch (key) {
    case "name":     return (t.name || "").toLowerCase();
    case "progress": return t.total > 0 ? t.downloaded / t.total : 0;
    case "speed":    return t.down_bps || 0;
    case "size":     return t.total_size || 0;
    case "added":
    default:         return t.added_at || 0;
  }
}

export function compareBy(key, dir) {
  const sign = dir === "asc" ? 1 : -1;
  return (a, b) => {
    const va = sortValue(a, key), vb = sortValue(b, key);
    if (va < vb) return -1 * sign;
    if (va > vb) return 1 * sign;
    return 0;
  };
}

export function sortTorrents(torrents, key, dir) {
  return [...torrents].sort(compareBy(key, dir));
}

// Human-friendly direction wording per sort key, shown on the active row of the
// sort menu. Arrow + a word so the direction is meaningful at a glance.
const DIR_WORDS = {
  added:    { desc: "newest", asc: "oldest" },
  name:     { desc: "Z–A",    asc: "A–Z" },
  progress: { desc: "high",   asc: "low" },
  speed:    { desc: "fast",   asc: "slow" },
  size:     { desc: "large",  asc: "small" },
};

export function sortDirectionLabel(key, dir) {
  const arrow = dir === "asc" ? "↑" : "↓";
  const word = DIR_WORDS[key]?.[dir];
  return word ? `${arrow} ${word}` : arrow;
}
