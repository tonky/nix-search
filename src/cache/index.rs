use std::path::Path;

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
    let nix_index = open_or_create(index_dir)?;
    let mut writer: IndexWriter = nix_index.index.writer(64_000_000)?;

    writer.delete_all_documents()?;
    for pkg in packages {
        writer.add_document(doc!(
            nix_index.fields.attr_path_exact => pkg.attr_path.as_str(),
            nix_index.fields.attr_path_text => pkg.attr_path.as_str(),
            nix_index.fields.pname => pkg.pname.as_str(),
            nix_index.fields.version => pkg.version.as_str(),
            nix_index.fields.description => pkg.description.as_str(),
            nix_index.fields.platforms => pkg.platforms.join(" "),
        ))?;
    }
    writer.commit()?;
    Ok(())
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
