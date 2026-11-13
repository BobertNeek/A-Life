// Detached experimental joint policy v1. No host admission or old-policy replacement.
#[derive(Clone, Debug)]
pub struct Policy {
    base: Vec<f32>,
    l: Vec<usize>,
    o: Vec<usize>,
    forced: Vec<Option<u8>>,
    temp: f32,
    cat: Vec<f64>,
    cat_log: Vec<f64>,
    q: Vec<f64>,
    qlog: Vec<f64>,
    all: Vec<f64>,
}
fn dist(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let m = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let z = x.iter().map(|v| (v - m).exp()).sum::<f64>().ln();
    let logs = x.iter().map(|v| (v - m) - z).collect::<Vec<_>>();
    (logs.iter().map(|v| v.exp()).collect(), logs)
}
fn lse(x: &[f64]) -> f64 {
    let m = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    m + x.iter().map(|v| (v - m).exp()).sum::<f64>().ln()
}
impl Policy {
    pub fn categories(&self) -> &[f64] {
        &self.cat
    }
    pub fn pairs(&self) -> &[f64] {
        &self.q
    }
    pub fn temperature(&self) -> f32 {
        self.temp
    }
    pub fn forced_slot(&self, i: usize) -> Option<u8> {
        self.forced[i]
    }
    pub fn new(
        base: &[f32],
        lm: u32,
        om: u32,
        forced: &[Option<u8>],
        potential: &[f32],
        temp: f32,
    ) -> Result<Self, &'static str> {
        let n = base.len();
        if n == 0
            || n > 32
            || forced.len() != n
            || !temp.is_finite()
            || temp <= 0.
            || base.iter().any(|x| !x.is_finite())
            || lm == 0
            || om == 0
            || lm & om != 0
        {
            return Err("contract");
        }
        let full = u32::MAX >> (32 - n);
        if (lm | om) & !full != 0 {
            return Err("mask");
        }
        let l = (0..n).filter(|i| lm & (1 << i) != 0).collect::<Vec<_>>();
        let o = (0..n).filter(|i| om & (1 << i) != 0).collect::<Vec<_>>();
        for (i, &slot) in forced.iter().enumerate() {
            if slot.is_some_and(|s| s > 5)
                || (l.contains(&i) && slot != Some(0))
                || (o.contains(&i) && slot != Some(1))
                || (!l.contains(&i) && !o.contains(&i) && matches!(slot, Some(0) | Some(1)))
            {
                return Err("forced slot partition");
            }
        }
        if potential.len() != l.len() * o.len() || potential.iter().any(|x| !x.is_finite()) {
            return Err("potential");
        }
        let maxbase = base.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
        let x = base
            .iter()
            .map(|v| (*v as f64 - maxbase) / temp as f64)
            .collect::<Vec<_>>();
        let (all, _) = dist(&x);
        let mut cx = vec![
            lse(&l.iter().map(|i| x[*i]).collect::<Vec<_>>()),
            lse(&o.iter().map(|i| x[*i]).collect::<Vec<_>>()),
        ];
        cx.extend(
            (0..n)
                .filter(|i| !l.contains(i) && !o.contains(i))
                .map(|i| x[i]),
        );
        let (cat, cat_log) = dist(&cx);
        // Center exact contrasts before temperature scaling. Each operand begins
        // as f32, but even f64 loses a small base term beside a large potential.
        let mut terms = Vec::with_capacity(l.len() * o.len());
        for (a, &i) in l.iter().enumerate() {
            for (b, &j) in o.iter().enumerate() {
                terms.push([
                    f64::from(base[i]),
                    f64::from(base[j]),
                    f64::from(potential[a * o.len() + b]),
                ]);
            }
        }
        let mut maximum = 0;
        for k in 1..terms.len() {
            if pair_contrast(terms[k], terms[maximum]) > 0. {
                maximum = k;
            }
        }
        let pair = terms
            .iter()
            .map(|&v| pair_contrast(v, terms[maximum]) / f64::from(temp))
            .collect::<Vec<_>>();
        let (q, qlog) = dist(&pair);
        Ok(Self {
            base: base.to_vec(),
            l,
            o,
            forced: forced.to_vec(),
            temp,
            cat,
            cat_log,
            q,
            qlog,
            all,
        })
    }
    pub fn category(&self, index: usize) -> usize {
        if self.l.contains(&index) {
            0
        } else if self.o.contains(&index) {
            1
        } else {
            2 + (0..index)
                .filter(|i| !self.l.contains(i) && !self.o.contains(i))
                .count()
        }
    }
    pub fn logprob(&self, c: usize, p: usize) -> f64 {
        self.cat_log[c] + self.qlog[p]
    }
    pub fn gradient(&self, c: usize, p: usize) -> (Vec<f64>, Vec<f64>) {
        let n = self.base.len();
        let mut b = vec![0.; n];
        let mut v = vec![0.; self.q.len()];
        let t = self.temp as f64;
        // derivative of marginalized group probability, not discarded latent index.
        let group = (0..n)
            .filter(|i| self.category(*i) == c)
            .collect::<Vec<_>>();
        let (_, logs) = dist(
            &group
                .iter()
                .map(|i| self.base[*i] as f64 / t)
                .collect::<Vec<_>>(),
        );
        for i in 0..n {
            b[i] = -self.all[i] / t
        }
        for (i, log) in group.iter().zip(logs) {
            b[*i] += log.exp() / t
        }
        for k in 0..v.len() {
            v[k] = ((if k == p { 1. } else { 0. }) - self.q[k]) / t;
            b[self.l[k / self.o.len()]] += v[k];
            b[self.o[k % self.o.len()]] += v[k]
        }
        (b, v)
    }
    pub fn entropy_gradient(&self) -> (f64, Vec<f64>, Vec<f64>) {
        let mut h = 0.;
        let mut b = vec![0.; self.base.len()];
        let mut v = vec![0.; self.q.len()];
        for c in 0..self.cat.len() {
            for p in 0..self.q.len() {
                let prob = self.cat[c] * self.q[p];
                if prob == 0. {
                    continue;
                }
                let lp = self.logprob(c, p);
                h -= prob * lp;
                let (gb, gv) = self.gradient(c, p);
                for i in 0..b.len() {
                    b[i] -= prob * (lp + 1.) * gb[i]
                }
                for i in 0..v.len() {
                    v[i] -= prob * (lp + 1.) * gv[i]
                }
            }
        }
        (h, b, v)
    }
    pub fn sample(&self, u: [f32; 3]) -> Result<(usize, usize, usize, usize), &'static str> {
        if u.iter().any(|x| !x.is_finite() || *x < 0. || *x >= 1.) {
            return Err("random");
        };
        let latent = old_global(&self.base, self.temp, u[0])?;
        let c = self.category(latent);
        let rows = (0..self.l.len())
            .map(|a| {
                self.q[a * self.o.len()..(a + 1) * self.o.len()]
                    .iter()
                    .sum()
            })
            .collect::<Vec<f64>>();
        let a = cdf(&rows, u[1] as f64);
        let row = &self.q[a * self.o.len()..(a + 1) * self.o.len()];
        let b = cdf(row, u[2] as f64);
        let rep = if c == 0 {
            self.l[a]
        } else if c == 1 {
            self.o[b]
        } else {
            latent
        };
        Ok((rep, self.l[a], self.o[b], latent))
    }
}
fn cdf(v: &[f64], u: f64) -> usize {
    let z = v.iter().sum::<f64>();
    let mut c = 0.;
    for (i, x) in v.iter().enumerate() {
        c += x;
        if u * z < c {
            return i;
        }
    }
    v.len() - 1
}
// Exact native base-global operation ordering, not regrouped category sampling.
pub fn old_global(logits: &[f32], temp: f32, random: f32) -> Result<usize, &'static str> {
    if logits.is_empty()
        || logits.len() > 32
        || !temp.is_finite()
        || temp <= 0.
        || !random.is_finite()
        || !(0. ..1.).contains(&random)
        || logits.iter().any(|x| !x.is_finite())
    {
        return Err("global contract");
    };
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.;
    let mut values = Vec::new();
    for (i, &l) in logits.iter().enumerate() {
        let e = ((l - max) / temp).exp();
        sum += e;
        values.push((i, e));
    }
    if !sum.is_finite() || sum <= 0. {
        return Err("global sum");
    };
    let mut cumulative = 0.;
    let mut selected = values.last().ok_or("empty")?.0;
    for &(i, e) in &values {
        cumulative += e;
        if random * sum < cumulative {
            selected = i;
            break;
        }
    }
    Ok(selected)
}

/// Error-free expansion of the six finite original terms. Intermediate sums
/// cannot overflow f64: operands are finite f32 and there are only six terms.
/// Small terms survive cancellation in an exact expansion. Its f64 collapse
/// may differ from the correctly rounded exact total by an ulp.
fn pair_contrast(a: [f64; 3], b: [f64; 3]) -> f64 {
    let mut partials = [0.; 6];
    let mut count = 0;
    for mut x in [a[0], a[1], a[2], -b[0], -b[1], -b[2]] {
        let mut next = 0;
        for k in 0..count {
            let mut y: f64 = partials[k];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0. {
                partials[next] = lo;
                next += 1;
            }
            x = hi;
        }
        partials[next] = x;
        count = next + 1;
    }
    partials[..count].iter().rev().sum()
}
