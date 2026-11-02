use grokking_ai_algorithms::{autodiff::*, diffusion::*, random::Rng, vision::*};

#[test]
fn group_normalization_has_correct_statistics_and_input_gradient() {
    let evaluate = |v: f64| {
        let mut t = Tape::new();
        let pixels = t.variables(&[v, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        let gamma = t.variables(&[1.0, 1.0]);
        let beta = t.variables(&[0.0, 0.0]);
        let x = FeatureMap::new(2, 2, 2, pixels.clone()).unwrap();
        let y = group_norm(&mut t, &x, 2, &gamma, &beta).unwrap();
        for group in y.values.chunks(4) {
            assert!(group.iter().map(|v| t.value(*v)).sum::<f64>().abs() < 1e-10);
            assert!(
                (group.iter().map(|v| t.value(*v).powi(2)).sum::<f64>() / 4.0 - 1.0).abs() < 1e-4
            );
        }
        t.backward(y.values[0]).unwrap();
        (t.value(y.values[0]), t.gradient(pixels[0]))
    };
    let numerical = (evaluate(1.000001).0 - evaluate(0.999999).0) / 0.000002;
    assert!((numerical - evaluate(1.0).1).abs() < 1e-6);
}

#[test]
fn pooling_and_leaky_relu_preserve_expected_values_and_gradients() {
    for maximum in [false, true] {
        let mut t = Tape::new();
        let pixels = t.variables(&[1.0, 2.0, 3.0, 4.0]);
        let x = FeatureMap::new(1, 2, 2, pixels.clone()).unwrap();
        let y = pool2d(&mut t, &x, maximum).unwrap();
        assert_eq!(t.value(y.values[0]), if maximum { 4.0 } else { 2.5 });
        t.backward(y.values[0]).unwrap();
        assert_eq!(t.gradient(pixels[0]), if maximum { 0.0 } else { 0.25 });
        assert_eq!(t.gradient(pixels[3]), if maximum { 1.0 } else { 0.25 });
    }
    let mut t = Tape::new();
    let x = t.constant(-2.0);
    let y = grokking_ai_algorithms::neural::Activation::LeakyRelu.apply(&mut t, x);
    t.backward(y).unwrap();
    assert_eq!(t.value(y), -0.02);
    assert_eq!(t.gradient(x), 0.01);
}

#[test]
fn convolution_and_transposed_convolution_match_hand_calculation() {
    let mut rng = Rng::new(2);
    let mut weights = Vec::new();
    let convolution = Conv2d::new(
        &mut weights,
        ConvConfig {
            input: 1,
            output: 1,
            kernel: 2,
            stride: 1,
            padding: 0,
            transpose: false,
        },
        &mut rng,
    )
    .unwrap();
    weights[..4].fill(1.0);
    weights[4] = 0.0;
    let mut t = Tape::new();
    let p = t.variables(&weights);
    let input = t.variables(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    let x = FeatureMap::new(1, 3, 3, input).unwrap();
    let y = convolution.forward(&mut t, &p, &x).unwrap();
    assert_eq!(
        y.values.iter().map(|v| t.value(*v)).collect::<Vec<_>>(),
        [12.0, 16.0, 24.0, 28.0]
    );
    let sum = t.sum(&y.values);
    t.backward(sum).unwrap();
    assert_eq!(t.gradient(p[0]), 12.0);
    let mut weights = Vec::new();
    let up = Conv2d::new(
        &mut weights,
        ConvConfig {
            input: 1,
            output: 1,
            kernel: 2,
            stride: 2,
            padding: 0,
            transpose: true,
        },
        &mut rng,
    )
    .unwrap();
    weights[..4].fill(1.0);
    weights[4] = 0.0;
    let mut t = Tape::new();
    let p = t.variables(&weights);
    let input = t.variables(&[1.0, 2.0, 3.0, 4.0]);
    let x = FeatureMap::new(1, 2, 2, input).unwrap();
    let y = up.forward(&mut t, &p, &x).unwrap();
    assert_eq!((y.height, y.width), (4, 4));
    assert_eq!(
        y.values.iter().map(|v| t.value(*v)).collect::<Vec<_>>(),
        [
            1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 3.0, 3.0, 4.0, 4.0
        ]
    );
}

#[test]
fn diffusion_schedules_obey_bounds_and_an_oracle_recovers_clean_data() {
    for schedule in [
        NoiseSchedule::Linear,
        NoiseSchedule::Cosine,
        NoiseSchedule::Sigmoid,
    ] {
        let d = Diffusion::new(20, schedule).unwrap();
        assert!(d.beta.iter().all(|v| *v > 0.0 && *v < 1.0));
        assert!(d.alpha_bar.windows(2).all(|v| v[0] > v[1]));
        let clean = [-0.7, 0.3, 0.8];
        let noise = [0.1, -1.0, 0.6];
        let noisy = d.forward(&clean, &noise, 0).unwrap();
        let recovered = d.reverse_step(&noisy, &noise, 0, &mut Rng::new(5)).unwrap();
        for (actual, expected) in recovered.iter().zip(clean) {
            assert!((actual - expected).abs() < 1e-10);
        }
    }
    assert_eq!(normalize_pixels(&[0, 255]), [-1.0, 1.0]);
    assert!(Diffusion::from_beta(vec![0.0]).is_err());
}

#[test]
fn unet_learns_noise_and_runs_reverse_diffusion_and_masked_editing() {
    let mut network = UNet::new(4).unwrap();
    let diffusion = Diffusion::new(8, NoiseSchedule::Linear).unwrap();
    let clean: Vec<_> = (0..16)
        .map(|i| if i % 4 < 2 { -0.7 } else { 0.7 })
        .collect();
    let mut rng = Rng::new(9);
    let noise: Vec<_> = (0..16).map(|_| rng.normal() * 0.5).collect();
    let noisy = diffusion.forward(&clean, &noise, 5).unwrap();
    let example = NoiseExample {
        noisy,
        noise,
        size: 4,
        timestep: 5,
        label: [1.0, 0.0, 0.0, 0.0],
    };
    let history = network.train_noise(&[example], 400, 0.04).unwrap();
    assert!(
        history[history.len() - 1] < history[0] * 0.5,
        "{} -> {}",
        history[0],
        history[history.len() - 1]
    );
    let output = diffusion
        .generate(&network, 4, &[1.0, 0.0, 0.0, 0.0], 42, None)
        .unwrap();
    assert_eq!(output.len(), 16);
    assert!(output.iter().all(|v| v.is_finite()));
    let mut mask = vec![true; 16];
    mask[..4].fill(false);
    let edited = diffusion
        .generate(
            &network,
            4,
            &[1.0, 0.0, 0.0, 0.0],
            42,
            Some((&clean, &mask)),
        )
        .unwrap();
    assert_eq!(&edited[..4], &clean[..4]);
    let (canvas, mask, w, h) = outpainting_canvas(&[0.3; 4], 2, 2, 1).unwrap();
    assert_eq!((w, h), (4, 4));
    let expanded = diffusion
        .generate(
            &network,
            w,
            &[1.0, 0.0, 0.0, 0.0],
            3,
            Some((&canvas, &mask)),
        )
        .unwrap();
    for (i, edit) in mask.iter().enumerate() {
        if !edit {
            assert_eq!(expanded[i], 0.3);
        }
    }
    let high = diffusion
        .high_resolution_fix(&network, &clean, 4, 2, &[1.0, 0.0, 0.0, 0.0], 5)
        .unwrap();
    assert_eq!(high.len(), 64);
    assert!(high.iter().all(|v| v.is_finite()));
}

#[test]
fn lora_learns_a_rank_one_change_while_base_is_frozen() {
    let base = vec![1.0, 0.0, 0.0, 1.0];
    let mut layer = Lora::new(base.clone(), 2, 2, 1, 1.0, 8).unwrap();
    let mut last = 0.0;
    for _ in 0..300 {
        let mut t = Tape::new();
        let p = t.variables(&layer.parameters);
        let mut losses = Vec::new();
        for (x, target) in [([1.0, 0.0], [2.0, 1.0]), ([0.0, 1.0], [1.0, 2.0])] {
            let input = t.variables(&x);
            let y = layer.forward(&mut t, &p, &input).unwrap();
            losses.push(t.mse(&y, &target).unwrap());
        }
        let loss = t.sum(&losses);
        last = t.value(loss);
        t.backward(loss).unwrap();
        t.sgd(&mut layer.parameters, &p, 0.08).unwrap();
    }
    assert!(last < 1e-5, "{last}");
    assert_eq!(layer.base, base);
}

#[test]
fn controlnet_zero_initialization_preserves_base_and_side_branch_trains() {
    let base = UNet::new(3).unwrap();
    let frozen = base.parameters.clone();
    let mut control = ControlNet::new(9).unwrap();
    let original = control.parameters.clone();
    let mut initial = 0.0;
    let mut last = 0.0;
    for iteration in 0..80 {
        let mut t = Tape::new();
        let p = t.variables(&base.parameters);
        let cp = t.variables(&control.parameters);
        let pixels = t.variables(&[0.4; 16]);
        let x = FeatureMap::new(1, 4, 4, pixels).unwrap();
        let residual = control.forward(&mut t, &cp, &x).unwrap();
        if iteration == 0 {
            assert!(residual.values.iter().all(|v| t.value(*v) == 0.0));
        }
        let y = base
            .forward_controlled(&mut t, &p, &x, 2, &[0.0; 4], Some(&residual))
            .unwrap();
        if iteration == 0 {
            let plain = base.forward(&mut t, &p, &x, 2, &[0.0; 4]).unwrap();
            for (a, b) in y.values.iter().zip(plain.values) {
                assert_eq!(t.value(*a), t.value(b));
            }
        }
        let loss = t.mse(&y.values, &[0.5; 16]).unwrap();
        last = t.value(loss);
        if iteration == 0 {
            initial = last;
        }
        t.backward(loss).unwrap();
        t.sgd(&mut control.parameters, &cp, 0.2).unwrap();
    }
    assert!(last < initial, "{initial} -> {last}");
    assert_ne!(control.parameters, original);
    assert_eq!(base.parameters, frozen);
}

#[test]
fn image_prompt_adapter_and_upscaler_are_trainable() {
    let mut adapter = ImagePromptAdapter::new(2, 2, 4).unwrap();
    let mut initial = 0.0;
    let mut last = 0.0;
    for i in 0..60 {
        let mut t = Tape::new();
        let p = t.variables(&adapter.parameters);
        let q = vec![t.variables(&[1.0, 0.3])];
        let image = vec![t.variables(&[0.5, 1.0])];
        let text = vec![t.variables(&[0.2, 0.4])];
        let output = adapter.forward(&mut t, &p, &q, &image, &text, 1.0).unwrap();
        let loss = t.mse(&output[0], &[1.0, 0.0]).unwrap();
        last = t.value(loss);
        if i == 0 {
            initial = last;
        }
        t.backward(loss).unwrap();
        t.sgd(&mut adapter.parameters, &p, 0.05).unwrap();
    }
    assert!(last < initial * 0.1);
    let mut upscaler = Upscaler::new(7).unwrap();
    let mut initial = 0.0;
    let mut last = 0.0;
    for i in 0..100 {
        let mut t = Tape::new();
        let p = t.variables(&upscaler.parameters);
        let values = t.variables(&[0.5; 4]);
        let x = FeatureMap::new(1, 2, 2, values).unwrap();
        let y = upscaler.forward(&mut t, &p, &x).unwrap();
        assert_eq!((y.height, y.width), (4, 4));
        let loss = t.mse(&y.values, &[0.5; 16]).unwrap();
        last = t.value(loss);
        if i == 0 {
            initial = last;
        }
        t.backward(loss).unwrap();
        t.sgd(&mut upscaler.parameters, &p, 0.08).unwrap();
    }
    assert!(last < initial * 0.1);
}

#[test]
fn frechet_statistics_and_canny_edges_have_known_results() {
    let a = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![2.0, 2.0]];
    assert!(frechet_feature_distance(&a, &a).unwrap() < 1e-8);
    let b: Vec<Vec<_>> = a
        .iter()
        .map(|r| r.iter().map(|v| v + 1.0).collect())
        .collect();
    assert!((frechet_feature_distance(&a, &b).unwrap() - 2.0).abs() < 1e-8);
    let image: Vec<_> = (0..49).map(|i| if i % 7 < 3 { 0.0 } else { 1.0 }).collect();
    let edges = canny(&image, 7, 7, 0.2, 0.4).unwrap();
    assert!(edges.iter().any(|v| *v));
    for (i, &edge) in edges.iter().enumerate() {
        if edge {
            assert!([2, 3].contains(&(i % 7)));
        }
    }
    assert!(
        canny(&[0.0; 49], 7, 7, 0.2, 0.4)
            .unwrap()
            .iter()
            .all(|v| !v)
    );
}
