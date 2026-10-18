use itertools::Itertools;
use petgraph::Direction;
use petgraph::acyclic::Acyclic;
use petgraph::acyclic::AcyclicEdgeError;
use petgraph::algo::toposort;
use petgraph::data::Build as _;
use petgraph::data::DataMapMut as _;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use rapidhash::HashMapExt as _;
use rapidhash::RapidHashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::iter::zip;

#[derive(Debug)]
pub(crate) struct Dag<Node, Edge> {
    inner: Acyclic<DiGraph<Node, Edge>>,
}

impl<Node, Edge: 'static> Dag<Node, Edge> {
    pub(crate) fn new() -> Self {
        Dag {
            inner: Acyclic::new(),
        }
    }

    pub(crate) fn add_node(&mut self, node: Node) -> NodeIndex {
        self.inner.add_node(node)
    }

    pub(crate) fn add_edge(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        edge: Edge,
    ) -> Result<(), AcyclicEdgeError<NodeIndex>> {
        self.inner.try_add_edge(from, to, edge).map(|_| ())
    }

    pub(crate) fn recursive_map<NewNode: Send + Sync>(
        self,
        mut function: impl FnMut(Node, ChildIterator<NewNode>) -> NewNode,
    ) -> Dag<NewNode, Edge> {
        // We need to collect here to not capture the DAG.
        let indexes = self
            .indexes_bottom_up()
            .map(|index| (index, self.children(index).collect_vec()))
            .collect_vec();

        let (nodes, edges) = self.inner.into_inner().into_nodes_edges();

        let mut nodes = nodes
            .into_iter()
            .map(|node| node.weight)
            .map(Some)
            .collect_vec();

        let mut new_nodes = (0..nodes.len()).map(|_| None).collect_vec();

        for (index, children) in indexes {
            let node = nodes
                .get_mut(index.index())
                .expect("the indexes should not be invalidated")
                .take()
                .expect("the node should not be taken yet due to the iteration order");

            let mut children = children.into_iter().map(|index| {
                new_nodes
                    .get(index.index())
                    .expect("the indexes should not be invalidated")
                    .as_ref()
                    .expect("the node should exist due to the iteration order")
            });
            let children = ChildIterator(&mut children);

            let new_node = function(node, children);

            *new_nodes
                .get_mut(index.index())
                .expect("the indexes should not be invalidated") = Some(new_node);
        }

        let mut dag = Dag::new();

        for node in new_nodes {
            let node = node.expect("all nodes should have been mapped");

            dag.add_node(node);
        }

        for edge in edges {
            let from = edge.source();
            let to = edge.target();
            let edge = edge.weight;

            dag.add_edge(from, to, edge)
                .expect("since the edges haven't moved they should no cycle");
        }

        dag
    }

    pub(crate) fn map<NewNode>(
        self,
        mut function: impl FnMut(Node) -> NewNode,
    ) -> Dag<NewNode, Edge> {
        // TODO: use try_ok which was stabilised 8h ago as of writing.
        let Ok(mapped) = self.try_map::<_, Infallible>(|node| Ok(function(node)));

        mapped
    }

    pub(crate) fn try_map<NewNode, Error>(
        self,
        mut function: impl FnMut(Node) -> Result<NewNode, Error>,
    ) -> Result<Dag<NewNode, Edge>, Error> {
        let (nodes, edges) = self.inner.into_inner().into_nodes_edges();

        let mut new = Dag::new();

        for node in nodes {
            let node = node.weight;

            let new_node = function(node)?;

            new.add_node(new_node);
        }

        for edge in edges {
            let from = edge.source();
            let to = edge.target();
            let edge = edge.weight;

            new.add_edge(from, to, edge)
                .expect("since the edges haven't moved they should no cycle");
        }

        Ok(new)
    }

    /// Walks a `Dag` by visiting all children of a node before it.
    /// The function at each node gets the result from its children.
    /// The returned iterator is over the results of the root nodes.
    pub(crate) fn try_reduce_leaves<Output, Error>(
        self,
        mut function: impl FnMut(Node, ChildIterator<Output>) -> Result<Output, Error>,
    ) -> Result<impl Iterator<Item = Output> + Send, Error>
    where
        Output: Send + Sync,
    {
        let mut outputs = RapidHashMap::new();

        // We need to collect here to not capture the DAG.
        let indexes = self
            .indexes_bottom_up()
            .map(|index| (index, self.children(index).collect_vec()))
            .collect_vec();

        let mut nodes = self.map(Some);

        for (index, children) in indexes {
            let node = nodes
                .get_mut(index)
                .expect("the indexes should not be invalidated")
                .take()
                .expect("the node should not be taken yet due to the iteration order");

            let mut children = children.into_iter().map(|index| &outputs[&index]);
            let children = ChildIterator(&mut children);

            let result = function(node, children)?;

            outputs.insert(index, result);
        }

        Ok(outputs.into_values())
    }

    fn get_mut(&mut self, index: NodeIndex) -> Option<&mut Node> {
        self.inner.node_weight_mut(index)
    }

    fn children(&self, index: NodeIndex) -> impl Iterator<Item = NodeIndex> {
        self.inner.neighbors_directed(index, Direction::Outgoing)
    }

    pub(crate) fn indexed_nodes(&self) -> impl Iterator<Item = (NodeIndex, &Node)> {
        zip(self.inner.node_indices(), self.inner.node_weights())
    }

    pub(crate) fn into_nodes(self) -> impl Iterator<Item = Node> {
        let (nodes, _edges) = self.inner.into_inner().into_nodes_edges();

        nodes.into_iter().map(|node| node.weight)
    }

    fn indexes_bottom_up(&self) -> impl Iterator<Item = NodeIndex> {
        // This seems to be for optimising graph traversal. I don't know why this is mandatory.
        let workspace = None;

        // This is top down.
        let topological_order =
            toposort(&self.inner, workspace).expect("acyclic graphs should not have cycles");

        topological_order.into_iter().rev()
    }
}

pub struct ChildIterator<'nodes, Node>(&'nodes mut (dyn Iterator<Item = &'nodes Node> + Send));

impl<'nodes, Node> Iterator for ChildIterator<'nodes, Node> {
    type Item = &'nodes Node;

    fn next(&mut self) -> Option<&'nodes Node> {
        let ChildIterator(inner) = self;

        inner.next()
    }
}
