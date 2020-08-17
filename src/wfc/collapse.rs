use rand::prelude::*;
use rand::thread_rng;
use std::collections::{BinaryHeap};
use hashbrown::{HashSet, HashMap};
use std::mem::replace;
use std::ops::Index;
use crate::graph::graph::{Rules, Graph, VertexIndex, Frequencies, Labels, LabelFrequencies, Rules2, Graph2};
use crate::wfc::observe::Observe;
use crate::wfc::propagate::Propagate;
use nalgebra::DVector;
use crate::multiset::{MultisetTrait, EntCache};


struct Collapse<'a> {
    heap: BinaryHeap<Observe>,
    gen_observe: HashSet<VertexIndex>,
    observed: HashSet<VertexIndex>,
    propagations: Vec<Propagate>,
    rng: &'a mut StdRng,
    rules: &'a Rules,
    frequencies: &'a Frequencies,
    out_graph: Graph,
}

impl Collapse<'_> {
    pub fn new<'a>(rng: &'a mut StdRng,
                   rules: &'a Rules,
                   frequencies: &'a Frequencies,
                   all_labels: &'a Labels,
                   out_graph: Graph) -> Collapse<'a> {
        let mut heap: BinaryHeap<Observe> = BinaryHeap::new();
        let gen_observe: HashSet<VertexIndex> = HashSet::new();
        let mut observed: HashSet<VertexIndex> = HashSet::new();
        let mut propagations: Vec<Propagate> = Vec::new();

        let vertices_iter = out_graph.vertices.iter().enumerate();

        // initialize heap and observed
        // todo:
        //  Should we even initialize the heap? Maybe it would be better
        //  to only make these Observe elements when necessary.
        for (_index, labels) in vertices_iter.clone() {
            let from_index = _index as i32;
            if labels.len() == 1 { observed.insert(from_index); }
            else { heap.push(Observe::new_fuzz(rng, &from_index, labels, frequencies)) }
        }

        // Ensure that output graph is fully propagated before starting loop.
        // Generate Propagates for every vertex whose label set is a proper
        // subset of the set of all labels.
        for (_index, labels) in vertices_iter {
            let from_index = _index as i32;
            assert!(labels.is_subset(all_labels));
            if labels != all_labels { // labels is proper subset of all_labels
                generate_propagations(&mut propagations, &observed, &out_graph, &from_index);
            }
        }

        Collapse {
            heap,
            gen_observe,
            observed,
            propagations,
            rng,
            rules,
            frequencies,
            out_graph,
        }
    }

    pub fn exec(&mut self) -> Option<Graph> {
        let heap = &mut self.heap;
        let observed = &mut self.observed;
        let out_graph = &mut self.out_graph;
        let gen_observe = &mut self.gen_observe;
        let propagations = &mut self.propagations;
        let frequencies = &self.frequencies;
        let rules = &self.rules;

        loop {
            if observed.len() == out_graph.vertices.len() || heap.is_empty() {
                return Some(replace(out_graph, Graph::empty()));
            }
            if propagations.is_empty() {
                gen_observe.drain().for_each(|index| {  // algo: 4.2
                    let labels = out_graph.vertices.index(index as usize);
                    heap.push(Observe::new(&index, labels, frequencies))
                });
                let observe = heap.pop().unwrap();
                if observed.contains(&observe.index) { continue }
                out_graph.observe(self.rng, &observe.index, frequencies);
                observed.insert(observe.index);
                generate_propagations(propagations, observed, out_graph, &observe.index);
            } else {
                let propagate = propagations.pop().unwrap();
                // todo:
                //  wrap constraint building in a function which returns
                //  Some(constraint) if the constraint subset is a proper
                //  subset of all_labels, else returns None. Also for caching.
                let constraint = out_graph.vertices
                    .index(propagate.from as usize).iter()
                    .fold(HashSet::new(), |mut cst, label| {
                        if let Some(set) = rules.get(&(propagate.direction, *label)) {
                            cst.extend(set);
                        }
                        cst
                    });
                if let Some(labels) = out_graph.constrain(&propagate.to, &constraint) { //🎸
                    if labels.is_empty() { return None }
                    else if labels.len() == 1 { observed.insert(propagate.to); }
                    else { gen_observe.insert(propagate.to); }
                    generate_propagations(propagations, observed, out_graph, &propagate.to);
                }
            }
        }
    }
}

struct Collapse2<'a> {
    heap: BinaryHeap<Observe>,
    gen_observe: HashSet<VertexIndex>,
    observed: HashSet<VertexIndex>,
    propagations: Vec<Propagate>,
    rng: &'a mut StdRng,
    rules: &'a Rules2,
    out_graph: Graph2,
}

impl Collapse2<'_> {
    pub fn new<'a>(rng: &'a mut StdRng,
                   rules: &'a Rules2,
                   all_labels: &'a LabelFrequencies,
                   out_graph: Graph2) -> Collapse2<'a> {
        let mut heap: BinaryHeap<Observe> = BinaryHeap::new();
        let gen_observe: HashSet<VertexIndex> = HashSet::new();
        let mut observed: HashSet<VertexIndex> = HashSet::new();
        let mut propagations: Vec<Propagate> = Vec::new();

        let vertices_iter = out_graph.vertices.iter().enumerate();


        // initialize heap and observed
        // todo:
        //  Should we even initialize the heap? Maybe it would be better
        //  to only make these Observe elements when necessary.
        for (_index, labels) in vertices_iter.clone() {
            let from_index = _index as i32;
            if labels.len() == 1 { observed.insert(from_index); }
            else { heap.push(Observe::new_fuzz2(rng, &from_index, labels)) }
        }

        // Ensure that output graph is fully propagated before starting loop.
        // Generate Propagates for every vertex whose label set is a proper
        // subset of the set of all labels.
        for (_index, labels) in vertices_iter {
            let from_index = _index as i32;
            assert!(labels.is_subset(all_labels));
            if labels != all_labels { // labels is proper subset of all_labels
                generate_propagations2(&mut propagations, &observed, &out_graph, &from_index);
            }
        }

        Collapse2 {
            heap,
            gen_observe,
            observed,
            propagations,
            rng,
            rules,
            out_graph,
        }
    }

    pub fn exec(&mut self) -> Option<Graph2> {
        let heap = &mut self.heap;
        let observed = &mut self.observed;
        let out_graph = &mut self.out_graph;
        let gen_observe = &mut self.gen_observe;
        let propagations = &mut self.propagations;
        let rules = &self.rules;

        loop {
            if observed.len() == out_graph.vertices.len() || heap.is_empty() {
                return Some(replace(out_graph, Graph2::empty()));
            }
            if propagations.is_empty() {
                gen_observe.drain().for_each(|index| {  // algo: 4.2
                    let labels = out_graph.vertices.index(index as usize);
                    heap.push(Observe::new2(&index, labels))
                });
                let observe = heap.pop().unwrap();
                if observed.contains(&observe.index) { continue }
                out_graph.observe(self.rng, &observe.index);
                observed.insert(observe.index);
                generate_propagations2(propagations, observed, out_graph, &observe.index);
            } else {
                let propagate = propagations.pop().unwrap();
                let constraint = constraint(out_graph.vertices.index(propagate.from as usize), &propagate.direction, rules);

                if let Some(labels) = out_graph.constrain(&propagate.to, &constraint) { //🎸
                    if labels.is_empty() { return None }
                    else if labels.len() == 1 { observed.insert(propagate.to); }
                    else { gen_observe.insert(propagate.to); }
                    generate_propagations2(propagations, observed, out_graph, &propagate.to);
                }
            }
        }
    }
}

use std::sync::RwLock;
use std::hash::{Hash, Hasher, BuildHasher};
use hashbrown::hash_map::DefaultHashBuilder;
use lazy_static::*;

fn hash_prop(labels: &LabelFrequencies, direction: &u16) -> u64 {
    let mut h = DefaultHashBuilder::new().build_hasher();
    labels.hash(&mut h);
    direction.hash(&mut h);
    h.finish()
}

lazy_static! {
    static ref CACHE: RwLock<HashMap<u64, DVector<u32>>> = RwLock::new(HashMap::with_capacity(32));
}

pub fn constraint(labels: &LabelFrequencies, direction: &u16, rules: &Rules2) -> DVector<u32> {
    // let hash = hash_prop(labels, direction);
    // if let Some(result) = CACHE.read().unwrap().get(&hash) {
    //     return result.clone();
    // }
    let result = labels.iter().enumerate()
        .fold(DVector::<u32>::zeros(labels.len()), |mut acc, (index, frq)| {
            if frq > &0 {
                if let Some(a) = rules.get(&(*direction, index as u32)) {
                    acc = acc.sup(&a)
                }
            }
            acc
        });
    // let mut write_cache = CACHE.write().unwrap();
    // (*write_cache).insert(hash, result.clone());
    result
}

// todo: change this to a pure function?
fn generate_propagations(propagations: &mut Vec<Propagate>, observed: &HashSet<VertexIndex>, out_graph: &Graph, from_index: &VertexIndex) {
    out_graph.connections(from_index).iter().for_each(|(to_index, direction)| {
        if !observed.contains(to_index) {
            propagations.push(Propagate::new(*from_index, *to_index, *direction))
        }
    });
}

fn generate_propagations2(propagations: &mut Vec<Propagate>, observed: &HashSet<VertexIndex>, out_graph: &Graph2, from_index: &VertexIndex) {
    out_graph.edges.index(from_index).iter().for_each(|(to_index, direction)| {
        if !observed.contains(to_index) {
            propagations.push(Propagate::new(*from_index, *to_index, *direction))
        }
    });
}

pub fn collapse(input_graph: &Graph, output_graph: Graph, seed: Option<u64>, tries: Option<u16>) -> Option<Graph> {
    let mut rng = StdRng::seed_from_u64(seed.unwrap_or(thread_rng().next_u64()));
    let tries = tries.unwrap_or(10);

    let rules = input_graph.rules();
    let frequencies = input_graph.frequencies();
    let all_labels = input_graph.all_labels();

    for _ in 0..tries {
        let mut collapse_try = Collapse::new(&mut rng, &rules, &frequencies, &all_labels, output_graph.clone());
        if let opt_graph @ Some(_) = collapse_try.exec() {
            return opt_graph;
        }
    }
    println!("CONTRADICTION!");
    None
}

pub fn collapse2(input_graph: &Graph2, output_graph: Graph2, seed: Option<u64>, tries: Option<u16>) -> Option<Graph2> {
    let mut rng = StdRng::seed_from_u64(seed.unwrap_or(thread_rng().next_u64()));
    let tries = tries.unwrap_or(10);

    let rules = input_graph.rules();
    let all_labels = input_graph.all_labels.clone();

    for _ in 0..tries {
        let mut collapse_try = Collapse2::new(&mut rng, &rules, &all_labels, output_graph.clone());
        if let opt_graph @ Some(_) = collapse_try.exec() {
            return opt_graph;
        }
    }
    println!("CONTRADICTION!");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::graph::Edges;
    use crate::utils::{hash_map, hash_set};
    use std::iter::FromIterator;

    fn simple_edges() -> Edges {
        hash_map(&[
            (0, vec![(1, 0), (3, 2)]),
            (1, vec![(0, 1), (2, 2)]),
            (2, vec![(3, 1), (1, 3)]),
            (3, vec![(0, 3), (2, 0)])
        ])
    }

    fn simple_vertices() -> Vec<Labels> {
        vec![
            hash_set(&[0, 1, 2]),
            hash_set(&[0, 1, 2]),
            hash_set(&[0, 1, 2]),
            hash_set(&[0, 1, 2])
        ]
    }

    fn simple_rules() -> Rules {
        hash_map(&[
            ((0, 0), hash_set(&[1])),
            ((0, 1), hash_set(&[2])),
            ((1, 1), hash_set(&[0])),
            ((1, 2), hash_set(&[1])),
            ((2, 0), hash_set(&[1])),
            ((2, 1), hash_set(&[2])),
            ((3, 1), hash_set(&[0])),
            ((3, 2), hash_set(&[1])),
        ])
    }

    #[test]
    fn test_constraint() {
        let a: DVector<u32> = DVector::from_row_slice(&[1, 0, 0]);
        let b: DVector<u32> = DVector::from_row_slice(&[0, 0, 1]);
        let c: DVector<u32> = DVector::from_row_slice(&[1, 1, 1]);
        let rules: Rules2 = hash_map(&[
            ((0, 0), a),
            ((0, 1), b),
            ((0, 2), c)
        ]);

        let labels: &LabelFrequencies = &DVector::from_row_slice(&[2, 4, 0]);
        let direction: &u16 = &0;

        let result = constraint(labels, direction, &rules);
        let expected = DVector::from_row_slice(&[1, 0, 1]);
        assert_eq!(result, expected)
    }

    #[test]
    fn test_new() {
        let mut rng = StdRng::seed_from_u64(10);

        let edges = simple_edges();
        let graph_vertices: Vec<Labels> = simple_vertices();
        let out_graph = Graph::new(graph_vertices, edges);

        let rules: Rules = simple_rules();
        let frequencies = hash_map(&[(0, 1), (1, 2), (2, 1)]);
        let all_labels = hash_set(&[0, 1, 2]);

        let collapse = Collapse::new(&mut rng,
                                     &rules,
                                     &frequencies,
                                     &all_labels,
                                     out_graph);

        assert_eq!(collapse.heap.len(), 4);
    }

    #[test]
    fn test_new_partial() {
        let mut rng = StdRng::seed_from_u64(10);

        let edges = simple_edges();
        let vertices: Vec<Labels> = vec![
            hash_set(&[0]),
            hash_set(&[0, 2]),
            hash_set(&[0, 1, 2]),
            hash_set(&[0, 1, 2])
        ];
        let out_graph = Graph::new(vertices, edges);

        let rules: Rules = simple_rules();
        let frequencies = hash_map(&[(0, 1), (1, 2), (2, 1)]);
        let all_labels = hash_set(&[0, 1, 2]);

        let collapse = Collapse::new(&mut rng,
                                     &rules,
                                     &frequencies,
                                     &all_labels,
                                     out_graph);

        assert_eq!(collapse.heap.len(), 3);
        assert_eq!(collapse.propagations.len(), 3);
        assert_eq!(collapse.observed, hash_set(&[0]));
    }

    #[test]
    fn test_exec_simple() {
        /* Seed Values:
            3 -> Some([{0}, {1}, {2}, {1}])
            1 -> None
        */
        let mut rng = StdRng::seed_from_u64(3);

        let edges = simple_edges();
        let vertices: Vec<Labels> = simple_vertices();
        let out_graph = Graph::new(vertices, edges);

        let rules: Rules = simple_rules();
        let frequencies = hash_map(&[(0, 1), (1, 2), (2, 1)]);
        let all_labels = hash_set(&[0, 1, 2]);

        let mut collapse = Collapse::new(&mut rng,
                                         &rules,
                                         &frequencies,
                                         &all_labels,
                                         out_graph);

        let result = collapse.exec().unwrap();
        let expected = Vec::from_iter(
            [0, 1, 2, 1].iter().map(|n: &i32| hash_set(&[*n]))
        );

        assert_eq!(result.vertices, expected);
    }

    #[test]
    fn test_exec_med() {
        /* Seed Values:
            INPUT:
            0b --- 1b --- 2a
            |      |      |
            3a --- 4b --- 5a

            Output structure same as input structure.
            North = 0, South = 1, East = 2, West = 3
        */
        let mut rng = StdRng::seed_from_u64(234);

        let edges = hash_map(&[
            (0, vec![(3, 1), (1, 2)]),
            (1, vec![(0, 3), (4, 1), (2, 2)]),
            (2, vec![(1, 3), (5, 1)]),
            (3, vec![(0, 0), (4, 2)]),
            (4, vec![(3, 3), (1, 0), (5, 2)]),
            (5, vec![(4, 3), (2, 0)])
        ]);

        let vertices: Vec<Labels> = vec![
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1])
        ];

        let out_graph = Graph::new(vertices, edges);

        let rules: Rules = hash_map(&[
            ((0, 0), hash_set(&[0, 1])),
            ((0, 1), hash_set(&[0, 1])),
            ((1, 0), hash_set(&[0, 1])),
            ((1, 1), hash_set(&[0, 1])),
            ((2, 0), hash_set(&[0, 1])),
            ((2, 1), hash_set(&[0, 1])),
            ((3, 0), hash_set(&[0, 1])),
            ((3, 1), hash_set(&[0, 1])),
        ]);
        let frequencies = hash_map(&[(0, 3), (1, 3)]);
        let all_labels = hash_set(&[0, 1]);

        let mut collapse = Collapse::new(&mut rng,
                                         &rules,
                                         &frequencies,
                                         &all_labels,
                                         out_graph);

        let result = collapse.exec().unwrap();
        let expected = Vec::from_iter(
            [1, 1, 0, 1, 1, 1].iter().map(|n: &i32| hash_set(&[*n]))
        );
        assert_eq!(result.vertices, expected);
    }

    #[test]
    fn test_exec_complex() {
        /*
            INPUT graph:
                   0a --- 1b --- 2b
                   |      | 
            3a --- 4a --- 5b

            Directions: North = 0, South = 1, East = 2, West = 3
        */

        let mut rng = StdRng::seed_from_u64(14392);

        let input_edges = hash_map(&[
            (0, vec![(1, 2), (4, 1)]),
            (1, vec![(0, 3), (5, 1), (2, 2)]),
            (2, vec![(1, 3)]),
            (3, vec![(4, 2)]),
            (4, vec![(3, 3), (0, 0), (5, 2)]),
            (5, vec![(4, 3), (1, 0)])
        ]);

        let input_vertices: Vec<Labels> = vec![
            hash_set(&[0]),
            hash_set(&[1]),
            hash_set(&[1]),
            hash_set(&[0]),
            hash_set(&[0]),
            hash_set(&[1])
        ];

        let input_graph = Graph::new(input_vertices, input_edges);

        let rules = input_graph.rules();
        let frequencies = input_graph.frequencies();
        let all_labels = input_graph.all_labels();

        /*
            OUTPUT STRUCTURE:
            0 ---- 1 ---- 2 ---- 3
            |      |      |      |
            4 ---- 5 ---- 6 ---- 7
            |      |      |      |
            8 ---- 9 ---- 10 --- 11
        */

        let output_edges: Edges = hash_map(&[
            (0, vec![(1, 2), (4, 1)]),
            (1, vec![(0, 3), (5, 1), (2, 2)]),
            (2, vec![(1, 3), (6, 1), (3, 2)]),
            (3, vec![(2, 3), (7, 1)]),
            (4, vec![(0, 0), (8, 1), (5, 2)]),
            (5, vec![(4, 3), (1, 0), (6, 2), (9, 1)]),
            (6, vec![(5, 3), (2, 0), (7, 2), (10, 1)]),
            (7, vec![(6, 3), (3, 0), (11, 1)]),
            (8, vec![(4, 0), (9, 2)]),
            (9, vec![(8, 3), (5, 0), (10, 2)]),
            (10, vec![(9, 3), (6, 0), (11, 2)]),
            (11, vec![(10, 3), (7, 0)])
        ]);

        let output_vertices: Vec<Labels> = vec![
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
            hash_set(&[0, 1]),
        ];

        let output_graph = Graph::new(output_vertices, output_edges);

        let mut collapse = Collapse::new(&mut rng,
                                         &rules,
                                         &frequencies,
                                         &all_labels,
                                         output_graph);

        let result = collapse.exec().unwrap();
        let expected = Vec::from_iter(
            [
                0, 0, 1, 1,
                0, 0, 1, 1,
                0, 0, 1, 1
            ].iter().map(|n: &i32| hash_set(&[*n]))
        );
        assert_eq!(result.vertices, expected);
    }
}
