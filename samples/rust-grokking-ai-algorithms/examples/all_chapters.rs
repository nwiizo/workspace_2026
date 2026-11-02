use grokking_ai_algorithms::{
    Result, clustering::*, diffusion::*, evolution::*, language::*, neural::*, random::Rng,
    reinforcement::*, search::*, supervised::*, swarm::*, vision::*,
};

fn main() -> Result<()> {
    search_demo()?;
    optimization_demo()?;
    learning_demo()?;
    language_demo()?;
    diffusion_demo()
}

fn search_demo() -> Result<()> {
    let mut graph = Graph::new(4);
    for (a, b) in [(0, 1), (0, 2), (1, 2), (2, 3)] {
        graph.add_edge(a, b)?;
    }
    let leaves = [3, 5, 6, 9, 1, 2, 0, -1];
    let (value, visited) = alpha_beta(&leaves, 3, true).ok_or("invalid tree")?;
    println!(
        "ch01-03: binary={:?}, BFS={:?}, DFS={:?}, minimax={value}, leaves={visited}/8",
        binary_search(&[1, 3, 5, 7], 5),
        graph.breadth_first_path(0, 3),
        graph.depth_first_path(0, 3)
    );
    Ok(())
}
fn optimization_demo() -> Result<()> {
    let (_, value) = knapsack(
        &[(2, 3), (3, 4), (4, 5), (5, 8)],
        7,
        GeneticConfig {
            population: 32,
            generations: 80,
            mutation: 0.1,
        },
        4,
    )?;
    let distances = vec![
        vec![0.0, 1.0, 2.0, 1.0],
        vec![1.0, 0.0, 1.0, 2.0],
        vec![2.0, 1.0, 0.0, 1.0],
        vec![1.0, 2.0, 1.0, 0.0],
    ];
    let (_, length) = ant_colony(
        &distances,
        AntConfig {
            ants: 12,
            iterations: 30,
            evaporation: 0.3,
            alpha: 1.0,
            beta: 2.0,
        },
        5,
    )?;
    let (position, loss) = particle_swarm(
        &[(-10.0, 10.0); 2],
        ParticleConfig {
            particles: 24,
            iterations: 160,
            inertia: 0.6,
            cognitive: 1.5,
            social: 1.5,
        },
        4,
        |x| booth(x[0], x[1]),
    )?;
    println!(
        "ch04-07: knapsack={value}, ACO tour={length}, PSO=({:.6},{:.6}), Booth={loss:.3e}",
        position[0], position[1]
    );
    Ok(())
}
fn learning_demo() -> Result<()> {
    let x = vec![
        vec![0.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 0.0],
        vec![1.0, 1.0],
    ];
    let tree = DecisionTree::fit(&x, &[0, 1, 1, 0], 3, Impurity::Gini)?;
    let predictions: Result<Vec<_>> = x.iter().map(|row| tree.predict(row)).collect();
    let points = vec![vec![0.0], vec![0.1], vec![5.0], vec![5.1]];
    let p = tsne_probabilities(&points, 1.5)?;
    let start = tsne(&points, 1.5, 0, 1.0, 3)?;
    let end = tsne(&points, 1.5, 300, 1.0, 3)?;
    println!(
        "ch08: CART XOR={:?}, t-SNE KL={:.6}->{:.6}",
        predictions?,
        tsne_loss_gradient(&p, &start)?.0,
        tsne_loss_gradient(&p, &end)?.0
    );
    let mut mlp = Mlp::new(&[2, 6, 1], Activation::Tanh, Activation::Sigmoid, 9)?;
    let targets = vec![vec![0.0], vec![1.0], vec![1.0], vec![0.0]];
    let loss = mlp.train_mse(&x, &targets, 1600, 0.6)?;
    println!(
        "ch09: MLP XOR MSE={:.6}->{:.6}",
        loss[0],
        loss[loss.len() - 1]
    );
    let mdp = Mdp {
        actions: vec![
            vec![
                Transition {
                    next: 0,
                    reward: -0.1,
                    terminal: false,
                },
                Transition {
                    next: 1,
                    reward: 0.0,
                    terminal: false,
                },
            ],
            vec![
                Transition {
                    next: 0,
                    reward: -0.1,
                    terminal: false,
                },
                Transition {
                    next: 2,
                    reward: 1.0,
                    terminal: true,
                },
            ],
            vec![],
        ],
    };
    let q = train_td(
        &mdp,
        0,
        TdConfig {
            episodes: 150,
            max_steps: 12,
            alpha: 0.2,
            gamma: 0.9,
            epsilon: 0.2,
        },
        false,
        8,
    )?;
    println!(
        "ch10: Q(start)=[{:.6},{:.6}], V*(start)={:.6}",
        q[0][0],
        q[0][1],
        value_iteration(&mdp, 0.9, 1e-9, 100)?[0]
    );
    Ok(())
}
fn language_demo() -> Result<()> {
    let bpe = Bpe::train("banana banana banana バナナ バナナ", 20);
    let text = "未知の文🦀";
    if bpe.decode(&bpe.encode(text))? != text {
        return Err("BPE roundtrip failed");
    }
    let mut model = Transformer::new(
        TransformerConfig {
            vocabulary: 3,
            width: 6,
            heads: 2,
            feed_forward: 10,
            layers: 2,
            context: 3,
        },
        8,
    )?;
    let before = model.loss(&[0, 1, 0], &[1, 0, 1])?;
    model.train(&[0, 1, 0, 1, 0, 1, 0, 1], 180, 0.07, 6)?;
    let after = model.loss(&[0, 1, 0], &[1, 0, 1])?;
    println!(
        "ch11: Transformer CE={before:.6}->{after:.6}, tokens={:?}, BPE Unicode roundtrip=ok",
        model.generate(&[0], 5, 0.05, 42)?
    );
    Ok(())
}
fn diffusion_demo() -> Result<()> {
    let mut model = UNet::new(4)?;
    let diffusion = Diffusion::new(8, NoiseSchedule::Linear)?;
    let clean: Vec<_> = (0..16)
        .map(|i| if i % 4 < 2 { -0.7 } else { 0.7 })
        .collect();
    let mut rng = Rng::new(9);
    let noise: Vec<_> = (0..16).map(|_| rng.normal() * 0.5).collect();
    let example = NoiseExample {
        noisy: diffusion.forward(&clean, &noise, 5)?,
        noise,
        size: 4,
        timestep: 5,
        label: [1.0, 0.0, 0.0, 0.0],
    };
    let loss = model.train_noise(&[example], 400, 0.04)?;
    let pixels = diffusion.generate(&model, 4, &[1.0, 0.0, 0.0, 0.0], 42, None)?;
    println!(
        "ch12: U-Net noise MSE={:.6}->{:.6}, generated={} pixels, finite={}",
        loss[0],
        loss[loss.len() - 1],
        pixels.len(),
        pixels.iter().all(|v| v.is_finite())
    );
    Ok(())
}
