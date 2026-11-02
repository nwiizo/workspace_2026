use grokking_ai_algorithms::{
    autodiff::*, language::*, neural::*, numeric::sigmoid, random::Rng, reinforcement::*,
};

#[test]
fn reverse_mode_matches_finite_differences_for_every_activation() {
    for activation in [
        Activation::Linear,
        Activation::Sigmoid,
        Activation::Tanh,
        Activation::Relu,
        Activation::Silu,
    ] {
        let value = 0.7;
        let h = 1e-6;
        let f = |x: f64| {
            let mut t = Tape::new();
            let x = t.constant(x);
            let y = activation.apply(&mut t, x);
            t.value(y)
        };
        let mut t = Tape::new();
        let x = t.constant(value);
        let y = activation.apply(&mut t, x);
        t.backward(y).unwrap();
        assert!((t.gradient(x) - (f(value + h) - f(value - h)) / (2.0 * h)).abs() < 1e-6);
    }
    let mut t = Tape::new();
    let x = t.constant(3.0);
    let square = t.mul(x, x);
    let repeated = t.add(square, square);
    t.backward(repeated).unwrap();
    assert_eq!(t.gradient(x), 12.0);
    assert_eq!(sigmoid(1000.0), 1.0);
    assert_eq!(sigmoid(-1000.0), 0.0);
}

#[test]
fn softmax_and_layernorm_gradients_match_finite_differences() {
    for normalized in [false, true] {
        let evaluate = |value: f64| {
            let mut t = Tape::new();
            let x = t.variables(&[value, -0.2, 0.9]);
            let output = if normalized {
                let gamma = t.variables(&[1.0; 3]);
                let beta = t.variables(&[0.0; 3]);
                layer_norm(&mut t, &x, &gamma, &beta).unwrap()
            } else {
                t.softmax(&x)
            };
            let loss = t.mse(&output, &[0.1, 0.2, 0.3]).unwrap();
            t.backward(loss).unwrap();
            (t.value(loss), t.gradient(x[0]))
        };
        let (loss, gradient) = evaluate(0.4);
        assert!(loss.is_finite());
        let numerical = (evaluate(0.400001).0 - evaluate(0.399999).0) / 0.000002;
        assert!(
            (gradient - numerical).abs() < 1e-6,
            "{gradient} {numerical}"
        );
    }
    let mut t = Tape::new();
    let logits = t.variables(&[1000.0, 1001.0]);
    let loss = t.cross_entropy(&logits, 1).unwrap();
    t.backward(loss).unwrap();
    assert!((t.value(loss) - 0.3132616875).abs() < 1e-9);
    assert!((t.gradient(logits[0]) + t.gradient(logits[1])).abs() < 1e-12);
}

#[test]
fn perceptron_learns_and_mlp_learns_xor() {
    let x = vec![
        vec![0.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 0.0],
        vec![1.0, 1.0],
    ];
    let and = Perceptron::fit(&x, &[false, false, false, true], 30, 0.1).unwrap();
    for (row, target) in x.iter().zip([false, false, false, true]) {
        assert_eq!(and.predict(row).unwrap(), target);
    }
    let mut model = Mlp::new(&[2, 6, 1], Activation::Tanh, Activation::Sigmoid, 9).unwrap();
    let targets = vec![vec![0.0], vec![1.0], vec![1.0], vec![0.0]];
    let history = model.train_mse(&x, &targets, 1600, 0.6).unwrap();
    assert!(
        history[history.len() - 1] < 0.02,
        "{}",
        history[history.len() - 1]
    );
    for (row, target) in x.iter().zip(targets) {
        assert_eq!(model.predict(row).unwrap()[0] > 0.5, target[0] > 0.5);
    }
}

#[test]
fn recurrent_networks_backpropagate_through_time() {
    let sequences = vec![vec![vec![-1.0], vec![0.0]], vec![vec![1.0], vec![0.0]]];
    let targets = vec![vec![-1.0], vec![1.0]];
    for kind in [RecurrentKind::Rnn, RecurrentKind::Lstm] {
        let mut network = Recurrent::new(1, 4, 1, kind, 3).unwrap();
        let history = network.train(&sequences, &targets, 250, 0.15).unwrap();
        assert!(
            history[history.len() - 1] < history[0] * 0.1,
            "{} -> {}",
            history[0],
            history[history.len() - 1]
        );
    }
}

#[test]
fn gan_updates_both_networks_toward_a_simple_distribution() {
    let mut model = Gan::new(5).unwrap();
    let g = model.generator.parameters.clone();
    let d = model.discriminator.parameters.clone();
    let noise = [-0.8, -0.2, 0.2, 0.8];
    let before = model.generator.predict(&[0.0]).unwrap()[0];
    for _ in 0..700 {
        let losses = model.train_step(&[2.0; 4], &noise, 0.025).unwrap();
        assert!(losses.0.is_finite() && losses.1.is_finite());
    }
    let after = model.generator.predict(&[0.0]).unwrap()[0];
    assert_ne!(model.generator.parameters, g);
    assert_ne!(model.discriminator.parameters, d);
    assert!(
        (after - 2.0).abs() < (before - 2.0).abs(),
        "{before} -> {after}"
    );
}

#[test]
fn bpe_roundtrips_unicode_and_unseen_text_and_batches_shift_targets() {
    let bpe = Bpe::train("banana banana banana バナナ バナナ", 20);
    for text in ["banana banana", "未知の文🦀", ""] {
        assert_eq!(bpe.decode(&bpe.encode(text)).unwrap(), text);
    }
    assert!(bpe.encode("banana banana").len() < "banana banana".len());
    assert_eq!(
        token_batches(&[0, 1, 2, 3], 2).unwrap(),
        [(vec![0, 1], vec![1, 2]), (vec![1, 2], vec![2, 3])]
    );
    assert!(token_batches(&[1], 0).is_err());
    assert_eq!(positional_encoding(0, 4), [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn attention_is_causal_and_query_gradients_are_correct() {
    let run = |query: f64, causal: bool| {
        let mut t = Tape::new();
        let q = vec![t.variables(&[query, 0.2]), t.variables(&[0.1, 0.4])];
        let k = vec![t.variables(&[0.3, 0.2]), t.variables(&[-0.2, 0.8])];
        let v = vec![t.variables(&[1.0, 2.0]), t.variables(&[9.0, 8.0])];
        let out = attention(&mut t, &q, &k, &v, causal).unwrap();
        let loss = t.sum(&out[0]);
        t.backward(loss).unwrap();
        (t.value(loss), t.gradient(q[0][0]))
    };
    assert_eq!(run(0.5, true).0, 3.0);
    let analytical = run(0.5, false).1;
    let numeric = (run(0.500001, false).0 - run(0.499999, false).0) / 0.000002;
    assert!((analytical - numeric).abs() < 1e-6);
}

fn transformer() -> Transformer {
    Transformer::new(
        TransformerConfig {
            vocabulary: 3,
            width: 6,
            heads: 2,
            feed_forward: 10,
            layers: 2,
            context: 3,
        },
        8,
    )
    .unwrap()
}
#[test]
fn transformer_trains_all_layers_and_cannot_read_future_tokens() {
    let mut model = transformer();
    let mut t = Tape::new();
    let p = t.variables(&model.parameters);
    let a = model.forward(&mut t, &p, &[0, 1, 2]).unwrap();
    let b = model.forward(&mut t, &p, &[0, 2, 1]).unwrap();
    for (left, right) in a[0].iter().zip(&b[0]) {
        assert_eq!(t.value(*left), t.value(*right));
    }
    let before = model.loss(&[0, 1, 0], &[1, 0, 1]).unwrap();
    let parameters = model.parameters.clone();
    model
        .train(&[0, 1, 0, 1, 0, 1, 0, 1], 180, 0.07, 6)
        .unwrap();
    let after = model.loss(&[0, 1, 0], &[1, 0, 1]).unwrap();
    assert!(after < before * 0.25, "{before} -> {after}");
    assert_ne!(
        &parameters[..18],
        &model.parameters[..18],
        "embeddings must train"
    );
    assert_ne!(
        &parameters[18..60],
        &model.parameters[18..60],
        "attention must train"
    );
    let generated = model.generate(&[0], 5, 0.05, 42).unwrap();
    assert_eq!(generated, [0, 1, 0, 1, 0, 1]);
}

#[test]
fn preferences_change_language_policy_and_moe_routes_sparse_experts() {
    let weights = fit_reward_model(&[vec![0.0, 1.0]], &[vec![1.0, 0.0]], 100, 0.1).unwrap();
    assert!(weights[1] > weights[0]);
    let mut model = transformer();
    let before = model.next_probabilities(&[0], 1.0).unwrap()[1];
    model
        .reinforce(&[0], &[weights[0], weights[1], weights[0]], 120, 0.03, 9)
        .unwrap();
    assert!(model.next_probabilities(&[0], 1.0).unwrap()[1] > before + 0.1);
    let experts = MixtureOfExperts::new(3, 4, 2, 9).unwrap();
    let mut t = Tape::new();
    let p = t.variables(&experts.parameters);
    let x = t.variables(&[1.0, 0.5, -0.2]);
    let (output, selected) = experts.forward(&mut t, &p, &x).unwrap();
    assert_eq!(selected.len(), 2);
    assert_ne!(selected[0], selected[1]);
    let loss = t.sum(&output);
    t.backward(loss).unwrap();
    assert!(p.iter().any(|v| t.gradient(*v) != 0.0));
    let (ids, prompt) = retrieval_prompt(
        &["Rust ownership borrowing", "cats sleep often"],
        "Rust ownership",
        1,
    )
    .unwrap();
    assert_eq!(ids, [0]);
    assert!(prompt.contains("Rust ownership borrowing"));
    assert!(!prompt.contains("cats"));
}

fn mdp() -> Mdp {
    Mdp {
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
    }
}
#[test]
fn tabular_and_neural_rl_learn_a_goal_reaching_policy() {
    let mdp = mdp();
    let config = TdConfig {
        episodes: 150,
        max_steps: 12,
        alpha: 0.2,
        gamma: 0.9,
        epsilon: 0.2,
    };
    let optimal = value_iteration(&mdp, 0.9, 1e-9, 100).unwrap();
    assert!((optimal[0] - 0.9).abs() < 1e-8);
    for on_policy in [false, true] {
        let q = train_td(&mdp, 0, config, on_policy, 8).unwrap();
        assert!(q[0][1] > q[0][0]);
        assert!(q[1][1] > q[1][0]);
    }
    assert_eq!(sarsa_update(2.0, 3.0, 4.0, 0.5, 0.8).unwrap(), 4.1);
    let network = train_dqn(&mdp, 0, config, 8).unwrap();
    let q = network.predict(&[1.0, 0.0, 0.0]).unwrap();
    assert!(q[1] > q[0], "{q:?}");
    let mut rng = Rng::new(0);
    assert_eq!(epsilon_greedy(&[1.0, 3.0], 0.0, &mut rng).unwrap(), 1);
}
