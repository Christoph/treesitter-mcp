use crate::common::format;

pub struct CompactOutput {
    // Keep the header around for debugging/consistency, but we don't currently
    // serialize it from this helper.
    _header: String,
    rows: Vec<String>,
}

impl CompactOutput {
    pub fn new(header: &str) -> Self {
        Self {
            _header: header.to_string(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, fields: &[&str]) {
        self.rows.push(format::format_row(fields));
    }

    pub fn rows_string(&self) -> String {
        self.rows.join("\n")
    }
}
