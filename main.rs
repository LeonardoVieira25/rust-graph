use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::time::Instant;

const MAX_CAPACITY: i32 = 100;

const ALFA_ITERATIONS: i32 = 100;
const RANDOM_ITERATIONS: i32 = 10;
const RANDOM_ALFA: bool = false;

// (id, x, y, demand)
type Node = (i32, i32, i32, i32);

type Nodes = Vec<Node>;

// (route, distance, capacity)
type Route = (Vec<Node>, f64, i32);

type Routes = Vec<Route>;

// (saving, new_route)
type Saving = (i32, Route);

type Savings = Vec<Saving>;

type DistanceMatrix = Vec<Vec<f64>>;

fn read_file() -> Result<String, io::Error> {
    let mut filename = String::new();

    println!("Enter filename: ");
    io::stdin().read_line(&mut filename)?;
    filename = filename.trim().to_string();

    let mut file = File::open(filename)?;

    if file.metadata()?.len() == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "File is empty"));
    }

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn parse_nodes(file_string: Result<String, io::Error>) -> Vec<Node> {
    let mut nodes: Nodes = Vec::new();

    if let Ok(content) = file_string {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut node_section = false;

        let mut demand_section = false;

        for line in lines {
            if line.starts_with("NODE_COORD_SECTION") {
                node_section = true;
                continue;
            }

            if line.starts_with("DEMAND_SECTION") {
                demand_section = true;
                node_section = false;
                continue;
            }

            if node_section {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 3 {
                    let id = parts[0].parse::<i32>().unwrap();
                    let x = parts[1].parse::<i32>().unwrap();
                    let y = parts[2].parse::<i32>().unwrap();
                    nodes.push((id, x, y, -1));
                }
            }
            if demand_section {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    let id = parts[0].parse::<i32>().unwrap();
                    let demand = parts[1].parse::<i32>().unwrap();
                    nodes[id as usize - 1].3 = demand;
                }
            }
        }
    }

    nodes
}

fn create_initial_routes(nodes: &Nodes, distance_matrix: &DistanceMatrix) -> Routes {
    let origin_node = nodes[0];

    let mut routes: Routes = Vec::new();

    for node in nodes {
        if node.0 == origin_node.0 {
            continue;
        }
        let mut route: Route = (
            Vec::new(),
            distance_matrix[origin_node.0 as usize - 1][node.0 as usize - 1] * 2.0,
            node.3,
        );
        route.0.push(origin_node);
        route.0.push(*node);
        route.0.push(origin_node);

        routes.push(route);
    }

    routes
}

fn is_node_equal(node1: &Node, node2: &Node) -> bool {
    node1.0 == node2.0
}

fn is_node_in_route(node: &Node, route: &Route) -> bool {
    route.0.iter().any(|n| is_node_equal(node, n))
}

fn _merge_routes(route1: &Route, route2: &Route, distance_matrix: &DistanceMatrix) -> Route {
    let origin_node: &Node = &route1.0[0];
    let mut new_route: Route = (Vec::new(), 0.0, route1.2 + route2.2);

    let mut all_nodes: Vec<&Node> = Vec::new();

    for node in &route1.0 {
        all_nodes.push(node);
    }
    for node in &route2.0 {
        all_nodes.push(node);
    }

    new_route.0.push(*origin_node);

    while new_route.0.len() < (all_nodes.len() - 3) {
        let mut min_distance = std::f64::MAX;
        let mut min_node: &Node = &all_nodes[0];

        for node in &all_nodes {
            if is_node_in_route(node, &new_route) {
                continue;
            }

            let last_node_index = new_route.0.last().unwrap().0 as usize;
            let distance = if last_node_index > 0 && node.0 > 0 {
                distance_matrix[last_node_index - 1][node.0 as usize - 1]
            } else {
                continue;
            };

            if distance < min_distance {
                min_distance = distance;
                min_node = node;
            }
        }

        new_route.0.push(*min_node);
        new_route.1 += min_distance;
    }

    // add the distance to the origin node
    new_route.1 +=
        distance_matrix[new_route.0.last().unwrap().0 as usize - 1][origin_node.0 as usize - 1];

    // add the origin node to the end of the route
    new_route.0.push(*origin_node);

    new_route
}

fn _calculate_savings_lento(routes: &Routes, distance_matrix: &DistanceMatrix) -> Savings {
    let mut savings: Savings = Vec::new();

    for i in 0..routes.len() {
        for j in i + 1..routes.len() {
            if i == j {
                break;
            }

            // verifica se a capacidade da rota é maior que a capacidade máxima
            if routes[i].2 + routes[j].2 > MAX_CAPACITY {
                continue;
            }

            // for each pair of routes
            let original_distance = routes[i].1 + routes[j].1;

            // merge routes
            // let new_route = _merge_routes(&routes[i], &routes[j], &distance_matrix);
            let new_route = _merge_routes_randomized(
                &routes[i],
                &routes[j],
                &distance_matrix,
                RANDOM_ITERATIONS,
            );

            let new_distance = new_route.1;

            let saving = original_distance - new_distance;

            if saving <= 0.0 {
                continue;
            }
            savings.push((saving as i32, new_route));
        }
    }

    savings
}
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use rand::Rng;
use rayon::prelude::*;

fn quickselect<'a>(arr: &'a mut [(f64, &'a Node)], k: usize) -> Option<(f64, &'a Node)> {
    let len = arr.len();
    if len == 0 {
        return None;
    }
    if k >= len {
        return None;
    }

    let pivot_index = partition(arr);
    if k == pivot_index {
        Some(arr[k])
    } else if k < pivot_index {
        quickselect(&mut arr[0..pivot_index], k)
    } else {
        quickselect(&mut arr[pivot_index + 1..len], k - pivot_index - 1)
    }
}

fn partition(arr: &mut [(f64, &Node)]) -> usize {
    let len = arr.len();
    let pivot_index = len / 2;
    arr.swap(pivot_index, len - 1);
    let mut i = 0;
    for j in 0..len - 1 {
        if arr[j].0 <= arr[len - 1].0 {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, len - 1);
    i
}
fn _merge_routes_randomized_stage(
    route1: &Route,
    route2: &Route,
    distance_matrix: &DistanceMatrix,
    alfa: f32,
    iterations: i32,
) -> Route {
    let mut best_result: Route = (Vec::new(), std::f64::MAX, 0);

    let origin_node: &Node = &route1.0[0];
    let mut all_nodes: Vec<&Node> = route1.0.iter().chain(route2.0.iter()).collect();
    all_nodes.retain(|&node| node.0 != origin_node.0);
    all_nodes.push(origin_node);

    for _i in 0..iterations {
        let mut new_route: Route = (Vec::new(), 0.0, route1.2 + route2.2);
        let mut nodes_in_route: HashSet<usize> = HashSet::new();

        new_route.0.push(*origin_node);
        nodes_in_route.insert(origin_node.0 as usize);

        while new_route.0.len() < all_nodes.len() {
            let mut node_distances: Vec<(f64, &Node)> = all_nodes
                .iter()
                .filter(|&node| !nodes_in_route.contains(&(node.0 as usize)))
                .map(|&node| {
                    let last_node_index = new_route.0.last().unwrap().0 as usize;
                    let distance = distance_matrix[last_node_index - 1][node.0 as usize - 1];
                    (distance, node)
                })
                .collect();

            let selected_node_index = (node_distances.len() as f32 * alfa) as usize;
            let selected_node = quickselect(&mut node_distances, selected_node_index).unwrap();

            new_route.0.push(*selected_node.1);
            new_route.1 += selected_node.0;
            nodes_in_route.insert(selected_node.1 .0 as usize);
        }

        new_route.1 +=
            distance_matrix[new_route.0.last().unwrap().0 as usize - 1][origin_node.0 as usize - 1];
        new_route.0.push(*origin_node);

        if new_route.1 < best_result.1 {
            best_result = new_route;
        }
    }

    best_result
}

fn _merge_routes_randomized(
    route1: &Route,
    route2: &Route,
    distance_matrix: &DistanceMatrix,
    iterations: i32,
) -> Route {
    let results: Vec<Route> = (1..iterations)
        // .into_par_iter()
        .into_iter()
        .map(|i| {
            let alfa = if RANDOM_ALFA {
                i as f32 / iterations as f32
            } else {
                rand::thread_rng().gen_range(0.0..1.0)
            };
            _merge_routes_randomized_stage(route1, route2, distance_matrix, alfa, ALFA_ITERATIONS)
        })
        .collect();

    results
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap()
        .clone()
}

fn calculate_savings(routes: &Routes, distance_matrix: &DistanceMatrix) -> Savings {
    let mut route_pairs: Vec<(usize, usize)> = Vec::new();

    for i in 0..routes.len() {
        for j in i + 1..routes.len() {
            if routes[i].2 + routes[j].2 > MAX_CAPACITY {
                continue;
            }
            route_pairs.push((i, j));
        }
    }

    let result: Savings = route_pairs
        .par_iter()
        .map(|pair| {
            let original_distance = routes[pair.0].1 + routes[pair.1].1;
            let new_route = _merge_routes_randomized(
                &routes[pair.0],
                &routes[pair.1],
                &distance_matrix,
                RANDOM_ITERATIONS,
            );
            let new_distance = new_route.1;
            let saving = original_distance - new_distance;
            (saving as i32, new_route)
        })
        .collect::<Vec<(i32, Route)>>();

    result
}

/*
args: vector of nodes
returns: distance matrix

distance_matrix: vector of vectors

*/
fn generate_distance_matrix(nodes: &Vec<(i32, i32, i32, i32)>) -> Vec<Vec<f64>> {
    let mut distance_matrix = Vec::new();

    for i in 0..nodes.len() {
        let mut row = Vec::new();
        for j in 0..nodes.len() {
            let distance = ((nodes[i].1 as f64 - nodes[j].1 as f64).powi(2)
                + (nodes[i].2 as f64 - nodes[j].2 as f64).powi(2))
            .sqrt();
            row.push(distance);
        }
        distance_matrix.push(row);
    }

    distance_matrix
}
fn generate_graph_file(nodes: &Nodes, routes: &Routes) {
    // open a results file
    let mut file = File::create("results.txt").unwrap();

    // write the header
    file.write_all(b"graph G {\n").unwrap();
    file.write_all(b"layout=\"fdp\";\n").unwrap();

    // position the nodes
    for node in nodes {
        let line = format!(
            "    {} [pos=\"{},{}!\", width=0, height=0, fillcolor=green, style=filled];\n",
            node.0, node.1, node.2
        );
        file.write_all(line.as_bytes()).unwrap();
    }

    // write the routes

    for route in routes {
        let mut line = String::from("    ");

        let mut prev_node = &route.0[0];

        for node in &route.0 {
            if prev_node.0 == node.0 {
                prev_node = node;
                continue;
            }
            line.push_str(&prev_node.0.to_string());
            line.push_str(" -- ");
            line.push_str(&node.0.to_string());
            line.push_str("[color=\"black\", penwidth=15];\n");
            prev_node = node;
        }

        file.write_all(line.as_bytes()).unwrap();
    }
    file.write_all(b"}").unwrap();
}

fn is_dominated(route1: &Route, route2: &Route) -> bool {
    for node in &route1.0 {
        if !is_node_in_route(node, route2) {
            return false;
        }
    }
    true
}

fn remove_routes_dominated(routes: &mut Routes, saving: &Saving) {
    let mut i = routes.len();
    while i > 0 {
        i -= 1;
        if is_dominated(&routes[i], &saving.1) {
            routes.remove(i);
        }
    }
}

fn main() {
    //*  graphs\B-n78-k10.vrp
    //*  graphs\A-n32-k5.vrp

    // read file
    let file_string = read_file();
    
    // Start timing
    let start = Instant::now();

    // parse nodes
    let nodes: Nodes = parse_nodes(file_string);

    // calculate distance matrix
    let distance_matrix: DistanceMatrix = generate_distance_matrix(&nodes);

    // put each node in its own route
    let mut routes: Routes = create_initial_routes(&nodes, &distance_matrix);

    let mut savings: Savings = calculate_savings(&routes, &distance_matrix);

    let savings_len = savings.len();
    let pb = ProgressBar::new(savings_len as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .progress_chars("#>-"),
    );

    while savings.len() > 0 {
        // get best saving
        let mut best_saving: Saving = (0, (Vec::new(), 0.0, 0));

        for saving in &savings {
            if saving.0 > best_saving.0 {
                best_saving = saving.clone();
            }
        }

        if best_saving.0 == 0 {
            break;
        }

        remove_routes_dominated(&mut routes, &best_saving);
        routes.push(best_saving.1);

        savings = calculate_savings(&routes, &distance_matrix);

        pb.inc(1);
        pb.set_length(savings.len() as u64);
    }

    // End timing
    let duration = start.elapsed();

    generate_graph_file(&nodes, &routes);

    pb.finish_with_message("completed");

    println!("Results: ");
    println!("n routes: {}", routes.len());
    println!(
        "total distance: {}",
        routes.iter().fold(0.0, |acc, route| acc + route.1)
    );
    println!(
        "total demand: {}",
        routes.iter().fold(0, |acc, route| acc + route.2)
    );
    for route in &routes {
        println!("route: {:?}", route);
    }
    // Print the duration in seconds
    println!("Time elapsed is: {:?}", duration);
}
