//! `validate --source` の複数 source file を一つの intent graph へ投影する。
//!
//! source file の順序と graph の登録順を固定し、同一 project 内の node identity を
//! source adapter の file 境界を越えて検査する。edge は全 node/evidence/review の登録後に
//! 追加するため、source file を跨ぐ endpoint も同じ graph へ解決できる。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lsharp_syntax::{ast::Program, span::Span};
use lsharp_types::{
    evidence::GraphError,
    validation::IntentGraph,
    validation_source::{self, SourceGraphError, SourceReviewAttestation},
};

use crate::error_codes::driver_io_error;

pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    pub(crate) source: String,
    pub(crate) program: Program,
}

pub(crate) struct SourceProject {
    pub(crate) graph: IntentGraph,
    pub(crate) attestations: Vec<SourceReviewAttestation>,
}

#[derive(Debug)]
pub(crate) enum SourceProjectError {
    SourceGraph {
        file_index: usize,
        error: SourceGraphError,
    },
    DuplicateNode {
        id: String,
        first_file_index: usize,
        first_span: Span,
        duplicate_file_index: usize,
        duplicate_span: Span,
    },
    Graph {
        file_index: usize,
        error: GraphError,
    },
}

pub(crate) fn collect_source_paths(source: &Path) -> miette::Result<Vec<PathBuf>> {
    let metadata = std::fs::symlink_metadata(source)
        .map_err(|error| driver_io_error(format!("{}: {error}", source.display())))?;
    if metadata.file_type().is_symlink() {
        return Err(driver_io_error(format!(
            "source tree must not contain symlinks: {}",
            source.display()
        )));
    }
    if metadata.is_file() {
        return Ok(vec![source.to_path_buf()]);
    }
    if !metadata.is_dir() {
        return Err(driver_io_error(format!(
            "source must be a regular .ls file or directory: {}",
            source.display()
        )));
    }

    let mut paths = Vec::new();
    collect_directory_paths(source, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_directory_paths(dir: &Path, out: &mut Vec<PathBuf>) -> miette::Result<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| driver_io_error(format!("{}: {error}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| driver_io_error(format!("{}: {error}", dir.display())))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| driver_io_error(format!("{}: {error}", path.display())))?;
        if metadata.file_type().is_symlink() {
            return Err(driver_io_error(format!(
                "source tree must not contain symlinks: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            collect_directory_paths(&path, out)?;
        } else if metadata.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("ls")
        {
            out.push(path);
        }
    }
    Ok(())
}

pub(crate) fn build(files: &[SourceFile]) -> Result<SourceProject, SourceProjectError> {
    let mut parsed = Vec::with_capacity(files.len());
    for (file_index, file) in files.iter().enumerate() {
        let (graph, attestations) =
            validation_source::source_program_to_intent_graph_with_attestations(&file.program)
                .map_err(|error| SourceProjectError::SourceGraph { file_index, error })?;
        parsed.push((graph, attestations));
    }

    let mut graph = IntentGraph::default();
    let mut node_locations = BTreeMap::<String, (usize, Span)>::new();
    for (file_index, (source_graph, _)) in parsed.iter().enumerate() {
        for node in source_graph.nodes() {
            let id = node.stable_id().as_str().to_owned();
            if let Some((first_file_index, first_span)) = node_locations.get(&id) {
                return Err(SourceProjectError::DuplicateNode {
                    id,
                    first_file_index: *first_file_index,
                    first_span: *first_span,
                    duplicate_file_index: file_index,
                    duplicate_span: node.source_span(),
                });
            }
            node_locations.insert(id, (file_index, node.source_span()));
            graph
                .add_node(node.clone())
                .map_err(|error| SourceProjectError::Graph { file_index, error })?;
        }
    }

    for (file_index, (source_graph, _)) in parsed.iter().enumerate() {
        for evidence in source_graph.evidence() {
            graph
                .add_evidence(evidence.clone())
                .map_err(|error| SourceProjectError::Graph { file_index, error })?;
        }
        for review in source_graph.reviews() {
            graph
                .add_review(review.clone())
                .map_err(|error| SourceProjectError::Graph { file_index, error })?;
        }
    }

    for (file_index, (source_graph, _)) in parsed.iter().enumerate() {
        for edge in source_graph.edges() {
            graph
                .add_edge(edge.clone())
                .map_err(|error| SourceProjectError::Graph { file_index, error })?;
        }
    }

    let attestations = parsed
        .into_iter()
        .flat_map(|(_, attestations)| attestations)
        .collect();
    Ok(SourceProject {
        graph,
        attestations,
    })
}
