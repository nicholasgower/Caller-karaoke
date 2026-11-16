//! A lightweight, per-category account of the app's resident memory.
//!
//! Users report the process growing to several hundred megabytes. Most of that
//! is deliberate: the bundled SoundFont, the rendered variation passes, and
//! optionally the Kokoro voice. This module measures the allocations the app
//! owns and marks the rest as estimates, so the Preferences table and
//! `docs/memory.md` can name the big consumers instead of guessing.
//!
//! Nothing here allocates a copy of the data it measures: buffers are counted
//! from their lengths, fonts from their atlas dimensions, and libraries from a
//! serialized size.

/// One row of the memory table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCategory {
    /// Stable category name, used by tests to look a row up.
    pub name: &'static str,
    /// Bytes attributed to this category.
    pub bytes: u64,
    /// True when the number is a calculation rather than an allocation the app
    /// can read the length of.
    pub estimated: bool,
    /// Short explanation shown under the table.
    pub note: &'static str,
}

/// Per-category memory use of one app state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryReport {
    categories: Vec<MemoryCategory>,
}

impl MemoryReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a row. Zero-byte optional consumers are kept: the table then shows
    /// that a category was considered and is currently free.
    pub fn add(&mut self, name: &'static str, bytes: u64, estimated: bool, note: &'static str) {
        self.categories.push(MemoryCategory {
            name,
            bytes,
            estimated,
            note,
        });
    }

    pub fn categories(&self) -> &[MemoryCategory] {
        &self.categories
    }

    /// Bytes in one category, or `None` when the report has no such row.
    pub fn bytes_for(&self, name: &str) -> Option<u64> {
        self.categories
            .iter()
            .find(|category| category.name == name)
            .map(|category| category.bytes)
    }

    pub fn total_bytes(&self) -> u64 {
        self.categories
            .iter()
            .fold(0u64, |total, category| total.saturating_add(category.bytes))
    }

    /// A plain-text table for the Maintenance section and test output.
    pub fn table(&self) -> String {
        const WIDTH: usize = 26;
        let mut text = format!("{:<WIDTH$} {:>10}  {}\n", "Category", "MiB", "Source");
        for category in &self.categories {
            let source = if category.estimated {
                "estimated"
            } else {
                "measured"
            };
            text.push_str(&format!(
                "{:<WIDTH$} {:>10.1}  {source}\n",
                category.name,
                mebibytes(category.bytes),
            ));
        }
        text.push_str(&format!(
            "{:<WIDTH$} {:>10.1}  total\n",
            "Total",
            mebibytes(self.total_bytes())
        ));
        text
    }

    /// One human-readable row, used by the Preferences grid so long notes can
    /// wrap at phone width instead of making a wide monospace block.
    pub fn rows(&self) -> impl Iterator<Item = (&'static str, u64, bool, &'static str)> + '_ {
        self.categories.iter().map(|category| {
            (
                category.name,
                category.bytes,
                category.estimated,
                category.note,
            )
        })
    }
}

/// Format bytes as mebibytes with one decimal.
pub fn mebibytes(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// Bytes in a vector's elements (not its allocation capacity).
pub fn vec_bytes<T>(values: &[T]) -> u64 {
    (values.len() as u64).saturating_mul(std::mem::size_of::<T>() as u64)
}

/// Resident bytes of an interleaved `f32` sample buffer.
pub fn sample_bytes(samples: usize) -> u64 {
    (samples as u64).saturating_mul(std::mem::size_of::<f32>() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_names_measured_and_estimated_rows_and_totals() {
        let mut report = MemoryReport::new();
        report.add("alpha", 2 * 1024 * 1024, false, "measured");
        report.add("beta", 1024 * 1024, true, "estimated");
        assert_eq!(report.bytes_for("alpha"), Some(2 * 1024 * 1024));
        assert_eq!(report.bytes_for("missing"), None);
        assert_eq!(report.total_bytes(), 3 * 1024 * 1024);
        let table = report.table();
        assert!(table.contains("alpha"));
        assert!(table.contains("estimated"));
        assert!(table.contains("Total"));
    }

    #[test]
    fn zero_rows_are_kept_so_a_disabled_consumer_is_visible() {
        let mut report = MemoryReport::new();
        report.add("neural_tts", 0, true, "off");
        assert_eq!(report.bytes_for("neural_tts"), Some(0));
        assert!(report.table().contains("neural_tts"));
    }

    #[test]
    fn sample_and_vec_sizes_use_the_element_width() {
        assert_eq!(sample_bytes(1_000), 4_000);
        assert_eq!(vec_bytes::<f32>(&[0.0; 1_000]), 4_000);
        assert_eq!(vec_bytes::<u8>(&[0; 10]), 10);
    }
}
