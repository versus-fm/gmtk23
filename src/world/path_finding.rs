use std::{slice::Iter, option::IntoIter, fmt::Display};

use bevy::prelude::{Vec2, Parent, Component};
use serde::__private::de;

use super::towers::{TowerField, SLOT_SIZE};


#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Node {
    pub x: i32,
    pub y: i32,
}


/* LinkedList 😱 */
#[derive(Debug, Clone)]
struct HierarchicalNode {
    pub node: Node,
    parent: Option<Box<HierarchicalNode>>,
    f: f32,
    g: f32
}



impl Node {
    pub fn new(x: i32, y: i32) -> Self {
        return Self { x, y }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "Node ( {}, {} )", self.x, self.y);
    }
}

impl HierarchicalNode {
    pub fn from_node(node: Node) -> Self {
        return Self {
            node,
            f: 0.,
            g: 0.,
            parent: None
        }
    }
    pub fn from_node_with_parent(node: Node, parent: &HierarchicalNode) -> Self {
        return Self {
            node,
            f: 0.,
            g: 0.,
            parent: Some(Box::new(parent.clone()))
        }
    }

    pub fn copy_to_node(&self) -> Node {
        return Node { x: self.node.x, y: self.node.y };
    }

    pub fn to_node(&self) -> Node {
        return self.node;
    }

    pub fn to_node_mut(&mut self) -> &mut Node {
        return &mut self.node;
    }
}

#[derive(Debug, Component)]
pub struct Path {
    route: Vec<Node>,
    current_index: usize
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{:?}", self.route);
    }
}

impl Path {
    pub fn empty() -> Self {
        return Self {
            route: Vec::new(),
            current_index: 0
        }
    }
    pub fn get_node(&self, index: usize) -> Node {
        return self.route[index];
    }

    pub fn get_size(&self) -> usize {
        return self.route.len();
    }

    pub fn get_target_position(&self) -> Vec2 {
        let node = self.get_node(self.current_index);
        let sizef = SLOT_SIZE as f32;
        return Vec2::new(node.x as f32 * sizef, node.y as f32 * sizef)
    }

    pub fn increment_index(&mut self) {
        if self.current_index < self.route.len() - 1 {
            self.current_index += 1;
        }
    }

    pub fn get_nodes(&self) -> Vec<Node> {
        return self.route.clone();
    }

    pub fn get_current_index(&self) -> usize {
        return self.current_index;
    }
}

pub fn a_star(field: &TowerField, start: Node, end: Node) -> Option<Path> {
    return a_star_with_blocked_node(field, start, end, None);
}

pub fn a_star_with_blocked_node(field: &TowerField, start: Node, end: Node, additional_blocked_node: Option<Node>) -> Option<Path> {
    if let Some(blocked) = additional_blocked_node {
        if start == blocked || end == blocked {
            return None;
        }
    }
    if is_outside_field(start, &field) {
        return None;
    }
    if is_outside_field(end, &field) {
        return None;
    }
    if field.is_node_blocked(start) {
        return None;
    }
    if field.is_node_blocked(end) {
        return None;
    }
    if start == end {
        return None;
    }

    let mut open: Vec<HierarchicalNode> = vec![HierarchicalNode::from_node(start)];
    let mut closed: Vec<HierarchicalNode> = Vec::new();

    while !open.is_empty() {
        match find_min_index(&open) {
            Some(min_f_index) => {
                let q = open[min_f_index].clone();
                open.remove(min_f_index);
                let successors = get_successors(q.to_node());
                for node in successors {
                    let mut successor = HierarchicalNode::from_node_with_parent(node, &q);
                    if successor.node == end {
                        return Some(get_path(successor));
                    }
                    if let Some(blocked) = additional_blocked_node {
                        if blocked == successor.node {
                            continue;
                        }
                    }
                    if is_outside_field(successor.to_node(), &field) {
                        continue;
                    }
                    if field.is_node_blocked(successor.to_node()) || contains_node(&closed, &successor) {
                        continue;
                    }
                    successor.g = q.g + 1.;
                    successor.f = successor.g + heuristic(successor.to_node(), end);
                    replace_if_better(&mut open, successor);
                }
                closed.push(q);
            },
            None => {
                return None;
            }
        }
    }
    return None;
}

pub fn get_successors(node: Node) -> [Node; 4] {
    return [
        Node::new(node.x - 1, node.y),
        Node::new(node.x + 1, node.y),
        Node::new(node.x, node.y + 1),
        Node::new(node.x, node.y - 1),
    ]
}

pub fn get_all_neighbors(node: Node) -> [Node; 8] {
    return [
        Node::new(node.x - 1, node.y),
        Node::new(node.x + 1, node.y),
        Node::new(node.x, node.y + 1),
        Node::new(node.x, node.y - 1),
        Node::new(node.x - 1, node.y - 1),
        Node::new(node.x + 1, node.y + 1),
        Node::new(node.x - 1, node.y + 1),
        Node::new(node.x + 1, node.y - 1),
    ]
}

pub fn get_self_with_successors(node: Node) -> [Node; 5] {
    return [
        node,
        Node::new(node.x - 1, node.y),
        Node::new(node.x + 1, node.y),
        Node::new(node.x, node.y + 1),
        Node::new(node.x, node.y - 1),
    ]
}

fn is_outside_field(node: Node, field: &TowerField) -> bool {
    // This !should! never panic because a tower field is *highly* unlikely to ever be over 2^31-1
    return node.x < 0 || node.x >= field.get_width().try_into().unwrap() || node.y < 0 || node.y >= field.get_height().try_into().unwrap();
}

fn contains_node(list: &Vec<HierarchicalNode>, node: &HierarchicalNode) -> bool {
    for i in 0..list.len() {
        if list[i].node == node.node {
            return true;
        }
    }
    return false;
}

fn find_min_index(list: &Vec<HierarchicalNode>) -> Option<usize> {
    if list.is_empty() {
        return None;
    }
    let mut min_index = usize::MAX;
    let mut min_f = f32::MAX;
    for i in 0..list.len() {
        let item = &list[i];
        if item.f < min_f {
            min_f = item.f;
            min_index = i;
        }
    }
    return Some(min_index);
}

fn replace_if_better(list: &mut Vec<HierarchicalNode>, new_node: HierarchicalNode) {
    let mut index: i32 = -1;
    let mut found = false;
    for i in 0..list.len() {
        if list[i].node == new_node.node && list[i].f > new_node.f {
            index = i as i32;
            break;
        } else if list[i].node == new_node.node {
            found = true;
        }
    }
    if index == -1 && !found {
        list.push(new_node);
    } else if index != -1 {
        list[index as usize] = new_node;
    }
}

fn get_path(destination: HierarchicalNode) -> Path {
    let mut path: Vec<Node> = Vec::new();
    let mut q = Some(&destination);
    while q.is_some() {
        path.insert(0, q.unwrap().copy_to_node());
        q = q.unwrap().parent.as_deref();
    }
    return Path {route: path, current_index: 0};
}


fn heuristic(node: Node, end: Node) -> f32 {
    return distance(node, end);
}

fn distance(from_node: Node, to_node: Node) -> f32 {
    return f32::abs((from_node.x - to_node.x) as f32) + f32::abs((from_node.y - to_node.y) as f32);
}