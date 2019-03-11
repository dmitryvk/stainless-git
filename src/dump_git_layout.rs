use crate::dag_layout::{Node, layout_nodes, CellId};

struct GitNode {
    id: git2::Oid,
    parents: Vec<git2::Oid>,
    msg: String,
}

impl Node for GitNode {
    type NodeId = git2::Oid;

    fn id(&self) -> git2::Oid {
        self.id
    }

    fn parents(&self) -> &[git2::Oid] {
        &self.parents
    }
}

pub fn dump_git_layout(repo_path: &std::ffi::OsString) {
    let repo = git2::Repository::discover(&repo_path).unwrap();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL/* | git2::Sort::REVERSE*/);
    revwalk.push_glob("*").unwrap();

    let nodes = revwalk.map(|oid_err|
        oid_err.and_then(|oid|
            repo.find_commit(oid).map(|commit|
                GitNode {
                    id: oid,
                    msg: String::from_utf8_lossy(commit.message_bytes()).trim().lines().nth(0).unwrap_or("").trim().to_owned(),
                    parents: commit.parent_ids().collect(),
                }
            )
        )
    ).collect::<Result<Vec<GitNode>, _>>().unwrap();

    let rows = layout_nodes(&nodes);

    for (row, node) in rows.iter().zip(nodes) {
        let mut line_top = String::new();
        let mut line_mid = String::new();
        let mut line_bot = String::new();

        for (i, _) in row.cells.iter().enumerate() {
            use std::fmt::Write;
            if row.active_cell == CellId(i as u32) {
                write!(&mut line_mid, " * ").unwrap();
            } else {
                write!(&mut line_mid, " | ").unwrap();
            }
        }

        let mut top_links_by_cell = vec![Vec::<usize>::new(); row.cells.len()];
        for (top_idx, mid_idx) in row.top_links.iter() {
            top_links_by_cell[mid_idx.0 as usize].push(top_idx.0 as usize);
        }
        
        for (i, links) in top_links_by_cell.iter().enumerate() {
            let has_less = links.iter().any(|l| *l < i);
            let has_eq = links.iter().any(|l| *l == i);
            let has_more = links.iter().any(|l| *l > i);
            
            use std::fmt::Write;
            write!(&mut line_top, "{}", if has_less { '\\' } else { ' ' }).unwrap();
            write!(&mut line_top, "{}", if has_eq { '|' } else { ' ' }).unwrap();
            write!(&mut line_top, "{}", if has_more { '/' } else { ' ' }).unwrap();
        }

        let mut bot_links_by_cell = vec![Vec::<usize>::new(); row.cells.len()];
        for (mid_idx, bot_idx) in row.bot_links.iter() {
            bot_links_by_cell[mid_idx.0 as usize].push(bot_idx.0 as usize);
        }
        
        for (i, links) in bot_links_by_cell.iter().enumerate() {
            let has_less = links.iter().any(|l| *l < i);
            let has_eq = links.iter().any(|l| *l == i);
            let has_more = links.iter().any(|l| *l > i);
            use std::fmt::Write;
            write!(&mut line_bot, "{}", if has_less { '/' } else { ' ' }).unwrap();
            write!(&mut line_bot, "{}", if has_eq { '|' } else { ' ' }).unwrap();
            write!(&mut line_bot, "{}", if has_more { '\\' } else { ' ' }).unwrap();
        }

        print!("{}\n{} {}\n{}\n", line_top, line_mid, node.msg, line_bot);
    }
}
