use neuro_ash::{BatchEvaluator, CpuEvaluator, DenseNetworkPool, Gpu, NetworkShape, Random, f16};

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 2.0e-4,
            "output {index}: got {actual}, expected {expected}"
        );
    }
}

#[test]
#[ignore = "requires a Vulkan device and loader"]
fn private_network_batch_matches_cpu() {
    let mut random = Random::new(21);
    let pool = DenseNetworkPool::random(NetworkShape::new(vec![5, 7, 4]).unwrap(), 3, &mut random);
    let inputs = (0..pool.len() * pool.shape().input_size())
        .map(|_| random.symmetric(1.0))
        .collect::<Vec<_>>();
    let mut cpu = CpuEvaluator;
    let expected = cpu.evaluate(&pool, &inputs).unwrap();
    let mut gpu = Gpu::new().unwrap();
    let actual = gpu.evaluate(&pool, &inputs).unwrap();

    assert_close(&actual, &expected);
}

#[test]
#[ignore = "requires a Vulkan device and loader"]
fn chunked_private_network_batch_matches_cpu() {
    let mut random = Random::new(23);
    let pool = DenseNetworkPool::random(NetworkShape::new(vec![5, 7, 4]).unwrap(), 7, &mut random);
    let inputs = (0..pool.len() * pool.shape().input_size())
        .map(|index| index as f32 / 17.0 - 0.5)
        .collect::<Vec<_>>();
    let mut cpu = CpuEvaluator;
    let expected = cpu.evaluate(&pool, &inputs).unwrap();
    let mut gpu = Gpu::with_max_batch_size(3).unwrap();

    let actual = gpu.evaluate(&pool, &inputs).unwrap();

    assert_close(&actual, &expected);
}

#[test]
#[ignore = "requires a Vulkan device and loader"]
fn cached_pool_tracks_parameter_changes_and_population_shrink() {
    let mut random = Random::new(25);
    let mut pool =
        DenseNetworkPool::random(NetworkShape::new(vec![5, 7, 4]).unwrap(), 8, &mut random);
    let mut gpu = Gpu::with_max_batch_size(8).unwrap();
    let mut cpu = CpuEvaluator;
    let mut inputs = (0..pool.len() * pool.shape().input_size())
        .map(|index| index as f32 / 23.0 - 0.5)
        .collect::<Vec<_>>();

    let initial = gpu.evaluate(&pool, &inputs).unwrap();
    assert_close(&initial, &cpu.evaluate(&pool, &inputs).unwrap());

    pool.network_biases_mut(1, 3).unwrap()[2] += f16::from_f32(1.25);
    let after_mutation = gpu.evaluate(&pool, &inputs).unwrap();
    assert_close(&after_mutation, &cpu.evaluate(&pool, &inputs).unwrap());
    assert_ne!(after_mutation, initial);

    for _ in 0..7 {
        pool.swap_remove(0).unwrap();
    }
    pool.shrink_to_fit();
    inputs.truncate(pool.shape().input_size());
    let after_shrink = gpu.evaluate(&pool, &inputs).unwrap();
    assert_close(&after_shrink, &cpu.evaluate(&pool, &inputs).unwrap());
}
