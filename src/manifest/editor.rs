use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::Path;

use super::Manifest;

pub fn format_paths_tuple(paths: &[String], base_indent: &str) -> String {
    let item_indent = format!("{base_indent}    ");
    if paths.is_empty() {
        return ".{}".to_string();
    }

    let mut out = String::from(".{\n");
    for path in paths {
        out.push_str(&item_indent);
        out.push('"');
        // Escape quotes and backslashes if needed
        for c in path.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                other => out.push(other),
            }
        }
        out.push_str("\",\n");
    }
    out.push_str(base_indent);
    out.push('}');
    out
}

pub fn apply_paths_update(manifest: &Manifest, new_paths: &[String]) -> Result<String> {
    let formatted_tuple = format_paths_tuple(new_paths, &manifest.base_indent);
    let updated_source = if let Some((start, end)) = manifest.paths_value_span {
        let mut res = String::with_capacity(manifest.source.len() + formatted_tuple.len());
        res.push_str(&manifest.source[..start]);
        res.push_str(&formatted_tuple);
        res.push_str(&manifest.source[end..]);
        res
    } else {
        // Insert .paths field before the last closing brace
        let (_top_start, top_end) = manifest.top_level_span;
        let before_end = &manifest.source[..top_end - 1];
        let after_end = &manifest.source[top_end - 1..];

        let field_str = format!("\n{}.paths = {},\n", manifest.base_indent, formatted_tuple);

        let mut res = String::with_capacity(manifest.source.len() + field_str.len());
        res.push_str(before_end);
        res.push_str(&field_str);
        res.push_str(after_end);
        res
    };

    // Pre-validation: re-parse to ensure the resulting source is 100% valid ZON
    let re_parsed = Manifest::parse_source(
        &updated_source,
        manifest.path.clone(),
        manifest.package_root.clone(),
    )
    .context("Validation failed on updated manifest syntax")?;

    let parsed_paths: Vec<String> = re_parsed.paths.into_iter().map(|p| p.value).collect();
    if parsed_paths != new_paths {
        bail!(
            "Internal sanity check failed: parsed paths {:?} do not match expected {:?}",
            parsed_paths,
            new_paths
        );
    }

    Ok(updated_source)
}

pub fn write_manifest_atomic(manifest_path: &Path, content: &str) -> Result<()> {
    let parent_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_file = tempfile::NamedTempFile::new_in(parent_dir).with_context(|| {
        format!(
            "Failed to create temporary file in {}",
            parent_dir.display()
        )
    })?;

    temp_file.write_all(content.as_bytes()).with_context(|| {
        format!(
            "Failed to write content to temporary file for {}",
            manifest_path.display()
        )
    })?;
    temp_file.flush().with_context(|| {
        format!(
            "Failed to flush temporary file for {}",
            manifest_path.display()
        )
    })?;

    temp_file.persist(manifest_path).with_context(|| {
        format!(
            "Failed to atomically replace manifest at {}",
            manifest_path.display()
        )
    })?;

    Ok(())
}
