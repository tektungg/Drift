// Monochrome line-icon set. Each value is inline SVG markup using
// currentColor so the surrounding element controls the color.
// All icons share viewBox 0 0 24 24, stroke-width 2.

const SVG = (body) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${body}</svg>`;

export const ICONS = {
  // file-type categories
  video:    SVG('<rect x="2" y="5" width="20" height="14" rx="2"/><path d="m10 9 5 3-5 3z" fill="currentColor" stroke="none"/>'),
  audio:    SVG('<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>'),
  image:    SVG('<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-5-5L5 21"/>'),
  document: SVG('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>'),
  archive:  SVG('<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>'),
  program:  SVG('<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2"/>'),
  folder:   SVG('<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>'),
  other:    SVG('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>'),

  // sidebar filters
  all:         SVG('<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>'),
  downloading: SVG('<path d="M12 3v14"/><path d="m6 11 6 6 6-6"/><path d="M5 21h14"/>'),
  seeding:     SVG('<path d="M12 21V7"/><path d="m6 13 6-6 6 6"/><path d="M5 3h14"/>'),
  completed:   SVG('<path d="M20 6 9 17l-5-5"/>'),
  paused:      SVG('<rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/>'),
  gear:        SVG('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"/>'),

  // empty-state hint icons + Drift wave glyph
  link: SVG('<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>'),
  plus: SVG('<path d="M12 5v14"/><path d="M5 12h14"/>'),
  wave: SVG('<path d="M2 8c2 0 2 2 4 2s2-2 4-2 2 2 4 2 2-2 4-2 2 2 4 2"/><path d="M2 14c2 0 2 2 4 2s2-2 4-2 2 2 4 2 2-2 4-2 2 2 4 2"/>'),
};

// Map a filename to a file-type category key.
const EXT = {
  video: "mp4 mkv avi mov wmv flv webm m4v mpg mpeg ts m2ts".split(" "),
  audio: "mp3 flac wav aac ogg m4a wma opus alac".split(" "),
  document: "pdf epub mobi doc docx xls xlsx ppt pptx txt rtf csv".split(" "),
  archive: "zip rar 7z tar gz bz2 xz".split(" "),
  program: "exe msi dmg deb rpm apk appimage iso img".split(" "),
  image: "jpg jpeg png webp gif bmp svg tiff raw heic".split(" "),
};

export function extToCategory(filename) {
  const m = /\.([a-z0-9]+)$/i.exec(String(filename || ""));
  if (!m) return null; // no recognizable extension
  const ext = m[1].toLowerCase();
  for (const cat of ["video", "audio", "program", "archive", "document", "image"]) {
    if (EXT[cat].includes(ext)) return cat;
  }
  return "other";
}

export function icon(key) { return ICONS[key] || ICONS.other; }
