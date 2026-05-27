use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Video,
    Audio,
    Documents,
    Compressed,
    Programs,
    Images,
    Other,
}

impl Category {
    pub fn folder_name(self) -> &'static str {
        match self {
            Category::Video => "Video",
            Category::Audio => "Audio",
            Category::Documents => "Documents",
            Category::Compressed => "Compressed",
            Category::Programs => "Programs",
            Category::Images => "Images",
            Category::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CategoryMap {
    pub video: Vec<String>,
    pub audio: Vec<String>,
    pub documents: Vec<String>,
    pub compressed: Vec<String>,
    pub programs: Vec<String>,
    pub images: Vec<String>,
}

impl Default for CategoryMap {
    fn default() -> Self {
        Self {
            video: split("mp4 mkv avi mov wmv flv webm m4v mpg mpeg ts m2ts"),
            audio: split("mp3 flac wav aac ogg m4a wma opus alac"),
            documents: split("pdf epub mobi doc docx xls xlsx ppt pptx txt rtf csv"),
            compressed: split("zip rar 7z tar gz bz2 xz"),
            programs: split("exe msi dmg deb rpm apk appimage iso img"),
            images: split("jpg jpeg png webp gif bmp svg tiff raw heic"),
        }
    }
}

fn split(s: &str) -> Vec<String> {
    s.split_whitespace().map(|x| x.to_ascii_lowercase()).collect()
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
}

pub fn classify_extension(ext: &str, map: &CategoryMap) -> Category {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    if map.video.iter().any(|e| e == &ext) { return Category::Video; }
    if map.audio.iter().any(|e| e == &ext) { return Category::Audio; }
    if map.programs.iter().any(|e| e == &ext) { return Category::Programs; }
    if map.compressed.iter().any(|e| e == &ext) { return Category::Compressed; }
    if map.documents.iter().any(|e| e == &ext) { return Category::Documents; }
    if map.images.iter().any(|e| e == &ext) { return Category::Images; }
    Category::Other
}

const PRIORITY: [Category; 7] = [
    Category::Video, Category::Audio, Category::Programs,
    Category::Compressed, Category::Documents, Category::Images, Category::Other,
];

pub fn resolve(files: &[FileEntry], map: &CategoryMap) -> Category {
    if files.is_empty() { return Category::Other; }
    if files.len() == 1 {
        let ext = Path::new(&files[0].path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        return classify_extension(ext, map);
    }
    let mut totals: std::collections::HashMap<Category, u64> = std::collections::HashMap::new();
    for f in files {
        let ext = Path::new(&f.path).extension().and_then(|e| e.to_str()).unwrap_or("");
        let c = classify_extension(ext, map);
        *totals.entry(c).or_insert(0) += f.size;
    }
    let max = *totals.values().max().unwrap_or(&0);
    PRIORITY.iter().copied()
        .find(|c| totals.get(c).copied().unwrap_or(0) == max)
        .unwrap_or(Category::Other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, size: u64) -> FileEntry { FileEntry { path: path.into(), size } }

    #[test]
    fn single_file_video() {
        let m = CategoryMap::default();
        assert_eq!(resolve(&[f("Movie.mkv", 1)], &m), Category::Video);
    }
    #[test]
    fn single_file_program_iso() {
        let m = CategoryMap::default();
        assert_eq!(resolve(&[f("ubuntu-24.04.iso", 1)], &m), Category::Programs);
    }
    #[test]
    fn multi_file_video_largest_wins() {
        let m = CategoryMap::default();
        let files = vec![
            f("release/movie.mkv", 4_000_000_000),
            f("release/subs.srt", 50_000),
            f("release/info.nfo", 1_000),
        ];
        assert_eq!(resolve(&files, &m), Category::Video);
    }
    #[test]
    fn multi_file_audio_album() {
        let m = CategoryMap::default();
        let files = vec![
            f("album/01.flac", 50_000_000),
            f("album/02.flac", 50_000_000),
            f("album/cover.jpg", 200_000),
        ];
        assert_eq!(resolve(&files, &m), Category::Audio);
    }
    #[test]
    fn unknown_extension_other() {
        let m = CategoryMap::default();
        assert_eq!(resolve(&[f("data.xyz", 1)], &m), Category::Other);
    }
    #[test]
    fn tiebreak_prefers_video_over_audio() {
        let m = CategoryMap::default();
        let files = vec![ f("a.mkv", 1_000), f("b.mp3", 1_000) ];
        assert_eq!(resolve(&files, &m), Category::Video);
    }
    #[test]
    fn folder_names_stable() {
        assert_eq!(Category::Video.folder_name(), "Video");
        assert_eq!(Category::Other.folder_name(), "Other");
    }
}
