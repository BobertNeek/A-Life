use super::*;
fn objective(h: &JointHead2056, s: [f32; 4], e: [[f32; 2]; 3]) -> (f64, Vec<f64>) {
    let v = h.forward(s, e).unwrap().score().value;
    let p = JointPolicy2054::new(
        &[1., 2., -1., 0.4, 1.5],
        3,
        12,
        &[Some(0), Some(0), Some(1), Some(1), Some(2)],
        &[0., v, 0., 0.],
        0.7,
    )
    .unwrap();
    (p.logprob(0, 1).unwrap(), p.gradient(0, 1).unwrap().1)
}
#[test]
fn detached_objective_chain_all14_adjoints() {
    let h = JointHead2056::fresh([0.7, -0.2, 0.4], 1.3, AdamWConfig::default()).unwrap();
    let s = [0.2, -0.1, 0.8, 0.7];
    let e = [[0.6, 0.2], [-0.2, 0.4], [0.1, -0.3]];
    let t = h.forward(s, e).unwrap();
    let a = h.backward(&t, objective(&h, s, e).1[1] as f32).unwrap();
    let eps = 0.001f32;
    let near = |x: f64, y: f32| assert!((x - f64::from(y)).abs() < 0.0002, "{x} vs {y}");
    for i in 0..4 {
        let (mut x, mut y) = (h.clone(), h.clone());
        if i < 3 {
            x.coefficients[i] += eps;
            y.coefficients[i] -= eps
        } else {
            x.amplitude += eps;
            y.amplitude -= eps
        }
        near(
            (objective(&x, s, e).0 - objective(&y, s, e).0) / f64::from(2. * eps),
            a.parameters[i],
        );
        let (mut x, mut y) = (s, s);
        x[i] += eps;
        y[i] -= eps;
        near(
            (objective(&h, x, e).0 - objective(&h, y, e).0) / f64::from(2. * eps),
            a.state[i],
        );
    }
    for i in 0..3 {
        for j in 0..2 {
            let (mut x, mut y) = (e, e);
            x[i][j] += eps;
            y[i][j] -= eps;
            near(
                (objective(&h, s, x).0 - objective(&h, s, y).0) / f64::from(2. * eps),
                a.effects[i][j],
            );
        }
    }
}
#[test]
fn synthetic_independent_torch_reference() {
    let p = JointPolicy2054::new(
        &[1., 2., -1., 0.4, 1.5],
        3,
        12,
        &[Some(0), Some(0), Some(1), Some(1), Some(2)],
        &[0.2, -0.1, 0.3, 0.7],
        0.7,
    )
    .unwrap();
    let expected = [0.6720221988376491, 0.06259435828003353, 0.26538344288231736];
    for (a, b) in p.categories().iter().zip(expected) {
        assert!((a - b).abs() < 1e-14);
    }
    assert!((p.logprob(0, 2).unwrap() - (-3.124810177053938)).abs() < 1e-14);
    assert!((p.entropy_gradient().0 - 1.3411147598083069).abs() < 1e-14);
}
