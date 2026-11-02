use grokking_ai_algorithms::{
    autodiff::*, diffusion::*, evolution::*, language::*, neural::*, training::*, vision::*,
};

#[test]
fn population_models_and_order_search_are_executable() {
    let config = GeneticConfig {
        population: 24,
        generations: 60,
        mutation: 0.1,
    };
    for model in [
        PopulationModel::Generational,
        PopulationModel::SteadyState(4),
        PopulationModel::Elitist(2),
    ] {
        let (bits, score) = binary_genetic_population(8, config, model, 3, |bits| {
            bits.iter().filter(|&&v| v).count() as f64
        })
        .unwrap();
        assert_eq!(bits.len(), 8);
        assert!(score >= 6.0);
    }
    let (order, cost) = genetic_order(5, config, 2, |order| {
        order.windows(2).map(|w| w[0].abs_diff(w[1]) as f64).sum()
    })
    .unwrap();
    assert_eq!(cost, 4.0);
    let mut sorted = order;
    sorted.sort_unstable();
    assert_eq!(sorted, [0, 1, 2, 3, 4]);
}

#[test]
fn neuroevolution_optimizes_neural_weights_without_backpropagation() {
    let template = Mlp::new(&[2, 1], Activation::Linear, Activation::Sigmoid, 1).unwrap();
    let data = [
        ([0.0, 0.0], 0.0),
        ([0.0, 1.0], 0.0),
        ([1.0, 0.0], 0.0),
        ([1.0, 1.0], 1.0),
    ];
    let (_, loss) = evolve_real(template.parameters.len(), (-8.0, 8.0), 300, 5, |weights| {
        let mut candidate = template.clone();
        candidate.parameters = weights.to_vec();
        data.iter()
            .map(|(row, target)| (candidate.predict(row).unwrap()[0] - target).powi(2))
            .sum::<f64>()
            / 4.0
    })
    .unwrap();
    assert!(loss < 0.03, "{loss}");
}

#[test]
fn checkpoint_roundtrip_preserves_predictions_and_rejects_corruption() {
    let model = Transformer::new(
        TransformerConfig {
            vocabulary: 3,
            width: 4,
            heads: 2,
            feed_forward: 8,
            layers: 1,
            context: 2,
        },
        7,
    )
    .unwrap();
    let checkpoint = transformer_checkpoint(&model).unwrap();
    let restored = restore_transformer(&checkpoint).unwrap();
    assert_eq!(restored.parameters, model.parameters);
    assert_eq!(
        restored.next_probabilities(&[0, 1], 1.0).unwrap(),
        model.next_probabilities(&[0, 1], 1.0).unwrap()
    );
    assert!(restore_transformer(&checkpoint.replace("RGA_TRANSFORMER_V1", "unknown")).is_err());
    assert!(restore_transformer(&checkpoint[..checkpoint.len() / 2]).is_err());
    assert!(
        restore_transformer("RGA_TRANSFORMER_V1\n999999999999 999999999999 1 1 1 1\n").is_err()
    );
}

#[test]
fn stopping_tuning_and_bigram_baseline_have_observable_results() {
    let mut stopping = EarlyStopping::new(2, 0.01).unwrap();
    assert!(!stopping.observe(1.0).unwrap());
    assert!(!stopping.observe(0.5).unwrap());
    assert!(!stopping.observe(0.51).unwrap());
    assert!(stopping.observe(0.52).unwrap());
    let (candidate, score) =
        grid_search(&[0.01_f64, 0.1, 0.5], |value| Ok((value - 0.1).powi(2))).unwrap();
    assert_eq!((candidate, score), (0.1, 0.0));
    let model = BigramModel::train(&[0, 1, 0, 1, 0, 1], 2).unwrap();
    assert!(model.probabilities(0).unwrap()[1] > model.probabilities(0).unwrap()[0]);
}

#[test]
fn cnn_classifier_learns_image_labels() {
    let mut model = CnnClassifier::new(2, 6).unwrap();
    let images = vec![vec![-0.8; 16], vec![-0.5; 16], vec![0.5; 16], vec![0.8; 16]];
    let history = model.train(&images, 4, &[0, 0, 1, 1], 100, 0.1).unwrap();
    assert!(history[history.len() - 1] < history[0] * 0.2);
    let mut t = Tape::new();
    let p = t.variables(&model.parameters);
    let pixels = t.variables(&[0.7; 16]);
    let input = FeatureMap::new(1, 4, 4, pixels).unwrap();
    let output = model.forward(&mut t, &p, &input).unwrap();
    assert!(t.value(output[1]) > t.value(output[0]));
}

#[test]
fn preference_reward_can_finetune_a_stochastic_denoising_policy() {
    let diffusion = Diffusion::from_beta(vec![0.05, 0.1, 0.15]).unwrap();
    let example = NoiseExample {
        noisy: vec![0.2; 16],
        noise: vec![0.0; 16],
        size: 4,
        timestep: 2,
        label: [0.0; 4],
    };
    let mut model = UNet::new(2).unwrap();
    let score = |model: &UNet| {
        let epsilon = model.predict(&example.noisy, 4, 2, &example.label).unwrap();
        let beta = diffusion.beta[2];
        let a = diffusion.alpha_bar[2];
        example
            .noisy
            .iter()
            .zip(epsilon)
            .map(|(x, e)| (x - beta * e / (1.0 - a).sqrt()) / (1.0 - beta).sqrt())
            .sum::<f64>()
            / 16.0
    };
    // Preferences choose brighter synthetic image features; no human data is claimed.
    let reward = fit_reward_model(&[vec![0.8; 16]], &[vec![-0.8; 16]], 20, 0.05).unwrap();
    let before = score(&model);
    reinforce_denoiser(&mut model, &diffusion, &example, &reward, 120, 0.001, 8).unwrap();
    assert!(score(&model) > before, "{before} -> {}", score(&model));
}
