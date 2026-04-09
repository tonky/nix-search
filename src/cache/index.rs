use std::path::Path;
use std::time::Instant;

use rayon::prelude::*;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, IndexWriter, TantivyDocument, doc};

use crate::types::Package;

#[derive(Debug, Clone, Copy)]
pub struct NixFields {
    pub attr_path_exact: Field,
    pub attr_path_text: Field,
    pub pname: Field,
    pub version: Field,
    pub description: Field,
    pub platforms: Field,
}

pub struct NixIndex {
    pub index: Index,
    pub schema: Schema,
    pub fields: NixFields,
}

const DEFAULT_WRITER_MEMORY_BYTES: usize = 128_000_000;
const DEFAULT_PARALLEL_DOC_THRESHOLD: usize = 10_000;

pub fn build_schema() -> (Schema, NixFields) {
    let mut builder = Schema::builder();
    let attr_path_exact = builder.add_text_field("attr_path_exact", STRING | STORED);
    let attr_path_text = builder.add_text_field("attr_path_text", TEXT | STORED);
    let pname = builder.add_text_field("pname", TEXT | STORED);
    let version = builder.add_text_field("version", STORED);
    let description = builder.add_text_field("description", TEXT | STORED);
    let platforms = builder.add_text_field("platforms", STORED);
    (
        builder.build(),
        NixFields {
            attr_path_exact,
            attr_path_text,
            pname,
            version,
            description,
            platforms,
        },
    )
}

pub fn open_or_create(index_dir: &Path) -> anyhow::Result<NixIndex> {
    let (schema, fields) = build_schema();
    let index = if index_dir.exists() {
        match Index::open_in_dir(index_dir) {
            Ok(i) => i,
            Err(_) => {
                std::fs::remove_dir_all(index_dir)?;
                std::fs::create_dir_all(index_dir)?;
                Index::create_in_dir(index_dir, schema.clone())?
            }
        }
    } else {
        std::fs::create_dir_all(index_dir)?;
        Index::create_in_dir(index_dir, schema.clone())?
    };

    Ok(NixIndex {
        index,
        schema,
        fields,
    })
}

pub fn build(index_dir: &Path, packages: &[Package]) -> anyhow::Result<()> {
    std::fs::create_dir_all(index_dir)?;

    let t_open_start = Instant::now();
    let nix_index = open_or_create(index_dir)?;
    let open_ms = t_open_start.elapsed().as_millis();

    let writer_memory_bytes = writer_memory_bytes();
    let mut writer: IndexWriter = nix_index.index.writer(writer_memory_bytes)?;

    let t_prepare_start = Instant::now();
    let docs = prepare_documents(&nix_index.fields, packages);
    let prepare_ms = t_prepare_start.elapsed().as_millis();

    let t_write_start = Instant::now();
    writer.delete_all_documents()?;
    for doc in docs {
        writer.add_document(doc)?;
    }
    let write_ms = t_write_start.elapsed().as_millis();

    let t_commit_start = Instant::now();
    writer.commit()?;
    let commit_ms = t_commit_start.elapsed().as_millis();

    eprintln!(
        "[perf][cache-index] packages={} open_ms={} prepare_ms={} write_ms={} commit_ms={} writer_mem_bytes={}",
        packages.len(),
        open_ms,
        prepare_ms,
        write_ms,
        commit_ms,
        writer_memory_bytes
    );

    Ok(())
}

fn writer_memory_bytes() -> usize {
    std::env::var("NIX_SEARCH_INDEX_WRITER_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v >= 16_000_000)
        .unwrap_or(DEFAULT_WRITER_MEMORY_BYTES)
}

fn parallel_doc_threshold() -> usize {
    std::env::var("NIX_SEARCH_INDEX_PARALLEL_DOC_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_PARALLEL_DOC_THRESHOLD)
}

fn package_to_doc(fields: &NixFields, pkg: &Package) -> TantivyDocument {
    doc!(
        fields.attr_path_exact => pkg.attr_path.as_str(),
        fields.attr_path_text => pkg.attr_path.as_str(),
        fields.pname => pkg.pname.as_str(),
        fields.version => pkg.version.as_str(),
        fields.description => pkg.description.as_str(),
        fields.platforms => pkg.platforms.join(" "),
    )
}

fn prepare_documents(fields: &NixFields, packages: &[Package]) -> Vec<TantivyDocument> {
    if packages.len() < parallel_doc_threshold() {
        return packages.iter().map(|pkg| package_to_doc(fields, pkg)).collect();
    }

    packages
        .par_iter()
        .map(|pkg| package_to_doc(fields, pkg))
        .collect()
}

pub fn doc_to_package(doc: &TantivyDocument, fields: &NixFields) -> Package {
    let get = |f: Field| {
        doc.get_first(f)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    Package {
        attr_path: get(fields.attr_path_exact),
        pname: get(fields.pname),
        version: get(fields.version),
        description: get(fields.description),
        platforms: get(fields.platforms)
            .split_whitespace()
            .map(str::to_string)
            .collect(),
    }
}

pub fn search_raw(index: &NixIndex, query_str: &str, limit: usize) -> anyhow::Result<Vec<Package>> {
    let reader = index.index.reader()?;
    let searcher = reader.searcher();

    let mut parser = QueryParser::for_index(
        &index.index,
        vec![
            index.fields.attr_path_text,
            index.fields.pname,
            index.fields.description,
        ],
    );
    parser.set_conjunction_by_default();

    let (query, _warnings) = parser.parse_query_lenient(query_str);
    let docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

    let mut out = Vec::with_capacity(docs.len());
    for (_, addr) in docs {
        let doc: TantivyDocument = searcher.doc(addr)?;
        out.push(doc_to_package(&doc, &index.fields));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{build_schema, doc_to_package, prepare_documents};
    use crate::types::Package;

    fn fixture_packages(n: usize) -> Vec<Package> {
        (0..n)
            .map(|i| Package {
                attr_path: format!("pkg.{i:05}"),
                pname: format!("pkg-{i:05}"),
                version: format!("1.{i}"),
                description: format!("fixture package {i}"),
                platforms: vec!["x86_64-linux".to_string(), "aarch64-darwin".to_string()],
            })
            .collect()
    }

    #[test]
    fn prepare_documents_is_deterministic_for_large_fixture() {
        let (_schema, fields) = build_schema();
        let pkgs = fixture_packages(25_000);

        let docs_a = prepare_documents(&fields, &pkgs);
        let docs_b = prepare_documents(&fields, &pkgs);

        let attrs_a = docs_a
            .iter()
            .map(|d| doc_to_package(d, &fields).attr_path)
            .collect::<Vec<_>>();
        let attrs_b = docs_b
            .iter()
            .map(|d| doc_to_package(d, &fields).attr_path)
            .collect::<Vec<_>>();

        assert_eq!(attrs_a, attrs_b);
    }
}
