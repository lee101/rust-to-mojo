//! Coverage reporting. Every top-level item and every `impl` method gets one
//! record. The ranked reason list is the work queue for later agent passes.

use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Translated with no outstanding TODOs.
    Translated,
    /// Code was emitted, but carries `TODO(rust-to-mojo)` notes (dropped
    /// bounds, unsupported derives, ...). Semantics of emitted code are
    /// believed faithful; the notes describe what is missing.
    Partial,
    /// Not translated: emitted as TODO + original Rust comment.
    Stubbed,
    /// Deliberately not translated (`#[cfg(test)]`, `#[test]`, ...).
    Skipped,
}

#[derive(Clone, Debug, Serialize)]
pub struct Reason {
    /// Normalised category used for ranking, e.g. "closure", "macro `write!`".
    pub key: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ItemRecord {
    pub kind: String,
    pub name: String,
    pub line: usize,
    pub status: Status,
    pub reasons: Vec<Reason>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Counts {
    pub total: usize,
    pub translated: usize,
    pub partial: usize,
    pub stubbed: usize,
    pub skipped: usize,
}

impl Counts {
    pub fn add(&mut self, s: Status) {
        self.total += 1;
        match s {
            Status::Translated => self.translated += 1,
            Status::Partial => self.partial += 1,
            Status::Stubbed => self.stubbed += 1,
            Status::Skipped => self.skipped += 1,
        }
    }
    pub fn merge(&mut self, o: &Counts) {
        self.total += o.total;
        self.translated += o.translated;
        self.partial += o.partial;
        self.stubbed += o.stubbed;
        self.skipped += o.skipped;
    }
    fn considered(&self) -> usize {
        self.total - self.skipped
    }
    /// Share of considered items that produced code (translated + partial).
    pub fn emitted_pct(&self) -> f64 {
        pct(self.translated + self.partial, self.considered())
    }
    /// Share of considered items that are fully clean.
    pub fn clean_pct(&self) -> f64 {
        pct(self.translated, self.considered())
    }
}

fn pct(a: usize, b: usize) -> f64 {
    if b == 0 {
        100.0
    } else {
        (a as f64) * 100.0 / (b as f64)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FileReport {
    pub source: String,
    pub output: String,
    pub module: String,
    pub counts: Counts,
    pub parse_error: Option<String>,
    pub file_notes: Vec<Reason>,
    pub items: Vec<ItemRecord>,
}

impl FileReport {
    pub fn push(&mut self, rec: ItemRecord) {
        self.counts.add(rec.status);
        self.items.push(rec);
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RankedReason {
    pub key: String,
    pub count: usize,
    pub example: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CrateReport {
    pub tool: String,
    pub mojo_target: String,
    pub crate_name: String,
    pub counts: Counts,
    pub emitted_pct: f64,
    pub clean_pct: f64,
    pub internal_errors: usize,
    /// Reasons that caused items to be stubbed, most frequent first.
    pub stub_reasons: Vec<RankedReason>,
    /// TODO notes on partially translated items, most frequent first.
    pub partial_reasons: Vec<RankedReason>,
    /// File-level notes (external crates, unresolved modules, ...).
    pub file_notes: Vec<RankedReason>,
    pub files: Vec<FileReport>,
}

fn rank<'a>(it: impl Iterator<Item = &'a Reason>) -> Vec<RankedReason> {
    let mut m: HashMap<&str, (usize, &str)> = HashMap::new();
    for r in it {
        let e = m.entry(r.key.as_str()).or_insert((0, r.detail.as_str()));
        e.0 += 1;
    }
    let mut v: Vec<RankedReason> = m
        .into_iter()
        .map(|(k, (c, ex))| RankedReason { key: k.to_string(), count: c, example: ex.to_string() })
        .collect();
    v.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
    v
}

impl CrateReport {
    pub fn build(crate_name: &str, files: Vec<FileReport>, internal_errors: usize) -> Self {
        let mut counts = Counts::default();
        for f in &files {
            counts.merge(&f.counts);
        }
        let stub_reasons = rank(
            files
                .iter()
                .flat_map(|f| f.items.iter())
                .filter(|i| i.status == Status::Stubbed)
                .flat_map(|i| i.reasons.iter().take(1)),
        );
        let partial_reasons = rank(
            files
                .iter()
                .flat_map(|f| f.items.iter())
                .filter(|i| i.status == Status::Partial)
                .flat_map(|i| i.reasons.iter()),
        );
        let file_notes = rank(files.iter().flat_map(|f| f.file_notes.iter()));
        CrateReport {
            tool: format!("rust-to-mojo {}", env!("CARGO_PKG_VERSION")),
            mojo_target: crate::MOJO_TARGET.to_string(),
            crate_name: crate_name.to_string(),
            emitted_pct: counts.emitted_pct(),
            clean_pct: counts.clean_pct(),
            counts,
            internal_errors,
            stub_reasons,
            partial_reasons,
            file_notes,
            files,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn summary(&self, top: usize) -> String {
        let mut s = String::new();
        let c = &self.counts;
        let _ = writeln!(s, "# rust-to-mojo coverage: `{}`\n", self.crate_name);
        let _ = writeln!(s, "Target: {}  ", self.mojo_target);
        let _ = writeln!(s, "Tool: {}\n", self.tool);
        let _ = writeln!(s, "| metric | value |\n| --- | ---: |");
        let _ = writeln!(s, "| files | {} |", self.files.len());
        let _ = writeln!(s, "| items (total) | {} |", c.total);
        let _ = writeln!(s, "| translated (clean) | {} |", c.translated);
        let _ = writeln!(s, "| partial (code + TODO notes) | {} |", c.partial);
        let _ = writeln!(s, "| stubbed (TODO + original Rust) | {} |", c.stubbed);
        let _ = writeln!(s, "| skipped (tests) | {} |", c.skipped);
        let _ = writeln!(s, "| emitted % (translated+partial / non-skipped) | {:.1}% |", self.emitted_pct);
        let _ = writeln!(s, "| clean % (translated / non-skipped) | {:.1}% |", self.clean_pct);
        let _ = writeln!(s, "| internal errors (panics caught) | {} |", self.internal_errors);
        let parse_errors = self.files.iter().filter(|f| f.parse_error.is_some()).count();
        let _ = writeln!(s, "| files with parse errors | {} |\n", parse_errors);

        let _ = writeln!(s, "## Top reasons items were stubbed\n");
        let _ = writeln!(s, "| # | reason | items | example |\n| ---: | --- | ---: | --- |");
        for (i, r) in self.stub_reasons.iter().take(top).enumerate() {
            let _ = writeln!(s, "| {} | {} | {} | {} |", i + 1, r.key, r.count, short(&r.example));
        }
        let _ = writeln!(s, "\n## Top TODO notes on partially translated items\n");
        let _ = writeln!(s, "| # | note | occurrences | example |\n| ---: | --- | ---: | --- |");
        for (i, r) in self.partial_reasons.iter().take(top).enumerate() {
            let _ = writeln!(s, "| {} | {} | {} | {} |", i + 1, r.key, r.count, short(&r.example));
        }
        if !self.file_notes.is_empty() {
            let _ = writeln!(s, "\n## File-level notes\n");
            let _ = writeln!(s, "| note | files |\n| --- | ---: |");
            for r in self.file_notes.iter().take(top) {
                let _ = writeln!(s, "| {} | {} |", r.key, r.count);
            }
        }
        let _ = writeln!(s, "\n## Per file\n");
        let _ = writeln!(s, "| source | items | translated | partial | stubbed | skipped | emitted % |");
        let _ = writeln!(s, "| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
        let mut files: Vec<&FileReport> = self.files.iter().collect();
        files.sort_by(|a, b| a.source.cmp(&b.source));
        for f in files {
            let fc = &f.counts;
            let _ = writeln!(
                s,
                "| {}{} | {} | {} | {} | {} | {} | {:.1}% |",
                f.source,
                if f.parse_error.is_some() { " (parse error)" } else { "" },
                fc.total,
                fc.translated,
                fc.partial,
                fc.stubbed,
                fc.skipped,
                fc.emitted_pct()
            );
        }
        s
    }
}

fn short(s: &str) -> String {
    let one = s.replace('\n', " ").replace('|', "\\|");
    if one.chars().count() > 90 {
        let t: String = one.chars().take(87).collect();
        format!("{}...", t)
    } else {
        one
    }
}
