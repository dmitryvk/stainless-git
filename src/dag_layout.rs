pub trait Node {
    type NodeId: std::fmt::Debug + Eq + PartialEq + Clone + std::hash::Hash + 'static;

    fn id(&self) -> Self::NodeId;
    fn parents(&self) -> &[Self::NodeId];
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CellId(pub u32);

#[derive(Debug, Clone)]
pub struct LayoutCell<TNode: Node> {
    pub id: TNode::NodeId,
}

#[derive(Debug, Clone)]
pub struct LayoutRow<TNode: Node> {
    pub cells: Vec<LayoutCell<TNode>>,
    pub top_links: Vec<(CellId, CellId)>,
    pub bot_links: Vec<(CellId, CellId)>,
    pub active_cell: CellId,
}

pub fn layout_nodes<TNode: Node>(nodes: &[TNode]) -> Vec<LayoutRow<TNode>> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let mut result = vec![
        LayoutRow { cells: vec![LayoutCell { id: nodes[0].id() }], top_links: Vec::new(), bot_links: Vec::new(), active_cell: CellId(0) }
    ];

    for node_pair in nodes.windows(2) {
        let prev_row = result.last_mut().expect("Value in vector is present since it is added even before the loop");
        let prev_node = node_pair.get(0).expect("Value exists since it comes from 2-window");
        let next_node = node_pair.get(1).expect("Value exists since it comes from 2-window");
        assert!(prev_row.cells.get(prev_row.active_cell.0 as usize).map(|x| Clone::clone(&x.id)) == Some(prev_node.id()));
        let next_row = compute_next_row(prev_row, prev_node, next_node);
        result.push(next_row);
    }

    return result;

    fn compute_next_row<TNode: Node>(prev_row: &mut LayoutRow<TNode>, prev_node: &TNode, next_node: &TNode) -> LayoutRow<TNode> {
        use std::collections::HashMap;

        let mut queue = Vec::<(TNode::NodeId, Option<usize>)>::new();

        for (prev_idx, prev_cell) in prev_row.cells.iter().enumerate() {
            if prev_cell.id == prev_node.id() {
                for parent_id in prev_node.parents().iter() {
                    queue.push((parent_id.clone(), Some(prev_idx)));
                }
            } else {
                queue.push((prev_cell.id.clone(), Some(prev_idx)));
            }
        }

        queue.push((next_node.id(), None));

        let mut cells = Vec::new();
        let mut cell_pos = HashMap::<TNode::NodeId, usize>::new();
        let mut top_links = Vec::new();

        for (node_id, parent_idx_opt) in &queue {
            let idx = *cell_pos.entry(node_id.clone()).or_insert_with(|| {
                cells.push(LayoutCell { id: node_id.clone() });
                cells.len() - 1
            });

            if let Some(parent_idx) = parent_idx_opt {
                top_links.push((CellId(*parent_idx as u32), CellId(idx as u32)));
            }
        }

        prev_row.bot_links = top_links.clone();

        LayoutRow {
            cells,
            active_cell: CellId(*cell_pos.get(&next_node.id()).expect("cell_pos contains all cells from row") as u32),
            top_links,
            bot_links: Vec::new(),
        }
    }
}
