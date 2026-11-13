use super::*;
fn policy() -> JointPolicy2054 {
    JointPolicy2054::new(
        &[1., 2., -1., 0.4, 1.5],
        3,
        12,
        &[Some(0), Some(0), Some(1), Some(1), Some(2)],
        &[0.2, -0.1, 0.3, 0.7],
        0.7,
    )
    .unwrap()
}
#[test]
fn public_indices_reject_instead_of_panicking() {
    let p = policy();
    for i in [5, usize::MAX] {
        assert!(p.category(i).is_err());
        assert!(p.forced_slot(i).is_err());
    }
    for (c, q) in [(3, 0), (0, 4), (usize::MAX, usize::MAX)] {
        assert!(p.logprob(c, q).is_err());
        assert!(p.gradient(c, q).is_err());
    }
}
#[test]
fn named_sample_and_observable_objective() {
    let p = policy();
    let s = p.sample([0.2, 0.3, 0.7]).unwrap();
    assert!(s.locomotion < 2);
    assert!((2..4).contains(&s.orientation));
    let c = p.category(s.representative).unwrap();
    let pair = s.locomotion * 2 + s.orientation - 2;
    assert!(p.logprob(c, pair).unwrap().is_finite());
    let (b, v) = p.gradient(c, pair).unwrap();
    assert_eq!((b.len(), v.len()), (5, 4));
    assert!(b.iter().chain(&v).all(|x| x.is_finite()));
}
#[test]
fn category_and_other_are_potential_independent() {
    let b = [1., 2., -1., 0.4, 1.5];
    let f = [Some(0), Some(0), Some(1), Some(1), Some(2)];
    let p = policy();
    for x in [-12288., 0., 12288.] {
        let q = JointPolicy2054::new(&b, 3, 12, &f, &[x, -x, x, -x], 0.7).unwrap();
        assert_eq!(p.categories(), q.categories());
        for i in 0..1000 {
            let u = [(i as f32 + 0.5) / 1000., 0.3, 0.7];
            let a = p.sample(u).unwrap();
            let z = q.sample(u).unwrap();
            assert_eq!(a.latent_global, z.latent_global);
            if a.latent_global == 4 {
                assert_eq!(a.representative, z.representative);
                assert_eq!(q.forced_slot(z.representative).unwrap(), Some(2));
            }
        }
    }
}
#[test]
fn boundary_shapes_and_uniforms() {
    let b = [0.; 32];
    let f = (0..32)
        .map(|i| Some(if i < 16 { 0 } else { 1 }))
        .collect::<Vec<_>>();
    let p = JointPolicy2054::new(&b, 65535, 0xffff0000, &f, &[0.; 256], 1.).unwrap();
    assert_eq!(p.pairs().len(), 256);
    for u in [[0.; 3], [f32::from_bits(1f32.to_bits() - 1); 3]] {
        p.sample(u).unwrap();
    }
    assert!(p.sample([1., 0., 0.]).is_err());
    assert!(p.sample([f32::NAN, 0., 0.]).is_err());
    assert!(JointPolicy2054::new(&[0.; 2], 1, 2, &[Some(0), Some(1)], &[0.], 1.).is_ok());
}
#[test]
fn public_entropy_and_temperature() {
    let p = policy();
    assert_eq!(p.temperature(), 0.7);
    let (h, b, v) = p.entropy_gradient();
    assert!(h > 0. && h.is_finite());
    assert!(b.iter().chain(&v).all(|x| x.is_finite()));
}
