//! Line-aware file chunking for indexing.
//!
//! Files are split into overlapping-free, line-bounded chunks sized by
//! character budget. Chunking happens server-side because the client uploads
//! whole files (`File { path, content }`) and the context engine is
//! responsible for turning them into embeddable [`Chunk`]s.

/// Default lower bound on chunk size in characters.
pub const DEFAULT_MIN_CHUNK_SIZE: usize = 512;

/// Default upper bound on chunk size in characters.
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 2048;

/// A contiguous, line-bounded slice of a file's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Path of the source file, relative to the workspace root.
    pub path: String,
    /// Verbatim text of the chunk.
    pub content: String,
    /// 1-based line number of the first line in the chunk.
    pub start_line: u32,
    /// 1-based line number of the last line in the chunk.
    pub end_line: u32,
}

/// Configuration controlling how files are split into chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkConfig {
    /// Soft lower bound: a chunk keeps accruing lines until it reaches at least
    /// this many characters (unless the file ends first).
    pub min_chunk_size: usize,
    /// Hard upper bound: a chunk is flushed once adding the next line would
    /// exceed this many characters (a single over-long line still forms its own
    /// chunk).
    pub max_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self { min_chunk_size: DEFAULT_MIN_CHUNK_SIZE, max_chunk_size: DEFAULT_MAX_CHUNK_SIZE }
    }
}

impl ChunkConfig {
    /// Builds a configuration, falling back to defaults for any bound given as
    /// `0` (the proto default for unset `uint32` fields), and guaranteeing
    /// `min <= max`.
    pub fn new(min_chunk_size: u32, max_chunk_size: u32) -> Self {
        let min =
            if min_chunk_size == 0 { DEFAULT_MIN_CHUNK_SIZE } else { min_chunk_size as usize };
        let max =
            if max_chunk_size == 0 { DEFAULT_MAX_CHUNK_SIZE } else { max_chunk_size as usize };
        Self { min_chunk_size: min, max_chunk_size: max.max(min) }
    }
}

/// Splits a file's content into line-bounded [`Chunk`]s.
///
/// Lines are accumulated into a chunk until the character budget is reached:
/// the current chunk is flushed once it is at least `min_chunk_size` characters
/// and adding the next line would push it over `max_chunk_size`. Empty files
/// (and files containing only whitespace) yield no chunks.
pub fn chunk_file(path: &str, content: &str, config: ChunkConfig) -> Vec<Chunk> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();

    let mut start_index = 0usize;
    let mut current_len = 0usize;
    let mut buffer: Vec<&str> = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        // +1 accounts for the newline that joins lines back together.
        let projected = current_len + line.len() + 1;

        let over_budget = projected > config.max_chunk_size;
        let big_enough = current_len >= config.min_chunk_size;

        if over_budget && big_enough && !buffer.is_empty() {
            chunks.push(build_chunk(path, &buffer, start_index));
            start_index = index;
            current_len = 0;
            buffer.clear();
        }

        buffer.push(line);
        current_len += line.len() + 1;
    }

    if !buffer.is_empty() {
        chunks.push(build_chunk(path, &buffer, start_index));
    }

    chunks
}

/// Assembles a [`Chunk`] from a buffer of consecutive lines.
fn build_chunk(path: &str, buffer: &[&str], start_index: usize) -> Chunk {
    let start_line = (start_index + 1) as u32;
    let end_line = (start_index + buffer.len()) as u32;
    Chunk {
        path: path.to_string(),
        content: buffer.join("\n"),
        start_line,
        end_line,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_empty_content_yields_no_chunks() {
        let actual = chunk_file("a.rs", "   \n  \n", ChunkConfig::default());
        let expected: Vec<Chunk> = Vec::new();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_small_file_is_single_chunk() {
        let content = "line one\nline two\nline three";
        let actual = chunk_file("a.rs", content, ChunkConfig::default());
        let expected = vec![Chunk {
            path: "a.rs".to_string(),
            content: content.to_string(),
            start_line: 1,
            end_line: 3,
        }];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_file_splits_on_budget_with_contiguous_lines() {
        // min=10, max=20 forces multiple chunks over a 6-line file.
        let config = ChunkConfig::new(10, 20);
        let content = "aaaaa\nbbbbb\nccccc\nddddd\neeeee\nfffff";
        let actual = chunk_file("f.txt", content, config);

        // Every line must be covered exactly once and ranges must be contiguous.
        assert_eq!(actual.first().unwrap().start_line, 1);
        assert_eq!(actual.last().unwrap().end_line, 6);
        for pair in actual.windows(2) {
            assert_eq!(pair[1].start_line, pair[0].end_line + 1);
        }
    }

    #[test]
    fn test_chunk_config_zero_falls_back_to_defaults() {
        let actual = ChunkConfig::new(0, 0);
        let expected = ChunkConfig::default();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_chunk_config_clamps_max_below_min() {
        let actual = ChunkConfig::new(1000, 100);
        assert_eq!(actual.max_chunk_size, 1000);
        assert_eq!(actual.min_chunk_size, 1000);
    }

    #[test]
    fn test_single_long_line_forms_one_chunk() {
        let long = "x".repeat(5000);
        let actual = chunk_file("big.txt", &long, ChunkConfig::new(512, 2048));
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].start_line, 1);
        assert_eq!(actual[0].end_line, 1);
    }
}
