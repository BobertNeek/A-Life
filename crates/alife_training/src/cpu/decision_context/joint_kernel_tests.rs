use super::kernel::*;
fn make(b: &[f32], v: &[f32], t: f32) -> Policy {
    Policy::new(
        b,
        3,
        12,
        &[Some(0), Some(0), Some(1), Some(1), Some(2)],
        v,
        t,
    )
    .unwrap()
}
#[test]
fn normalization_other() {
    for t in [0.1, 1., 3.] {
        let b = [1., 2., -1., 0.4, 1.5];
        let p = make(&b, &[0.; 4], t);
        for scale in [-1024., 0., 1024.] {
            let q = make(&b, &[scale, -scale, 0., scale], t);
            assert_eq!(p.categories(), q.categories());
            assert!((q.pairs().iter().sum::<f64>() - 1.).abs() < 1e-13);
            for i in 0..1000 {
                let u = [(i as f32 + 0.5) / 1000., 0.3, 0.7];
                let a = p.sample(u).unwrap();
                let c = q.sample(u).unwrap();
                assert_eq!(a.3, c.3);
                if a.3 == 4 {
                    assert_eq!(a.0, c.0);
                    assert_eq!(q.forced_slot(c.0), Some(2))
                }
            }
        }
    }
}
#[test]
fn finite_derivatives() {
    let b = [1., 2., -1., 0.4, 1.5];
    let v = [0.2, -0.1, 0.3, 0.7];
    for temp in [0.3, 1., 3.] {
        let p = make(&b, &v, temp);
        let (gb, gv) = p.gradient(0, 2);
        let (_, hb, hv) = p.entropy_gradient();
        for axis in 0..9 {
            let mut bp = b;
            let mut bm = b;
            let mut vp = v;
            let mut vm = v;
            let delta = if axis < 5 {
                bp[axis] += 0.001;
                bm[axis] -= 0.001;
                (bp[axis] - bm[axis]) as f64
            } else {
                vp[axis - 5] += 0.001;
                vm[axis - 5] -= 0.001;
                (vp[axis - 5] - vm[axis - 5]) as f64
            };
            let a = make(&bp, &vp, temp);
            let c = make(&bm, &vm, temp);
            let g = (a.logprob(0, 2) - c.logprob(0, 2)) / delta;
            let h = (a.entropy_gradient().0 - c.entropy_gradient().0) / delta;
            assert!((g - if axis < 5 { gb[axis] } else { gv[axis - 5] }).abs() < 1e-5);
            assert!((h - if axis < 5 { hb[axis] } else { hv[axis - 5] }).abs() < 1e-5)
        }
    }
}
#[test]
fn extreme() {
    for b in [[f32::MAX; 5], [f32::MAX, -f32::MAX, 0., 0., f32::MAX]] {
        let p = make(&b, &[f32::MAX; 4], f32::MIN_POSITIVE);
        assert!((p.categories().iter().sum::<f64>() - 1.).abs() < 1e-13);
        assert!((p.pairs().iter().sum::<f64>() - 1.).abs() < 1e-13);
        let (h, b, v) = p.entropy_gradient();
        assert!(h.is_finite() && b.iter().chain(&v).all(|x| x.is_finite()));
        p.sample([0.1, 0.2, 0.3]).unwrap();
    }
}
#[test]
fn invalid() {
    let b = [0.; 5];
    let f = [Some(0), Some(0), Some(1), Some(1), Some(2)];
    for (l, o) in [(0, 12), (3, 3), (3, 32)] {
        assert!(Policy::new(&b, l, o, &f, &[0.; 4], 1.).is_err())
    }
    assert!(Policy::new(&b, 3, 12, &f, &[0.; 4], 0.).is_err());
    assert!(Policy::new(&b, 3, 12, &[Some(0); 5], &[0.; 4], 1.).is_err());
    assert!(make(&b, &[0.; 4], 1.).sample([1., 0., 0.]).is_err());
}
