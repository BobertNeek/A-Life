# Spatial navigation: held-out and causal follow-up

Fresh CPU experimental lives only. No accepted trained life, optimizer, or game default was replaced. Receipt archives are local research data; their SHA-256 digests below identify them, not publicly hosted files.

## Held-out real-physics mazes

For each seed and variant, target food was initially occluded, the world ran for 240 decisions, and consumption was measured from actual outcomes. Seeds 1843, 1844, 1845: each of nav-turn10 and nav-boost ate hidden food in 2/3 right-gap runs and 0/3 left-gap runs. On seed 1845 nav-turn10 crossed the right wall end but did not eat. Cold next-step checks were exact. Seed 1843: each variant ate clear and neutral food and did not consume the bitter hazardous object (configured pain 0.3, observed maximum pain delta 0) in this one control set. These are tiny samples, not robust maze skill or broad food-safety proof.

Archives: 1843 maze/clear `39a0aa9a048137aa9c257b09834f38295e7183e1fe757cf64834d724340ecdad`; 1843 food `e15d7337350f9b067ed2469a6062679b514a5cf8f0ce8b4eee541040d8256f69`; 1844 `de62c53440baf0d17c6ea2e1581c0f54a4024d4dd7c9244e7c4253829317e908`; 1845 `5017240478ec24b9be2e4c198cc2bf451e25cfa6a6bcb27e6a7cc441a424aa47`.

## Causal recurrent-state intervention

A temporary CPU-only research port routed direction of a currently visible blue-biased object into the existing recurrent brain. Blue was a fixture, not universal food recognition. At a completed tick after four visible decisions, paired branches turned the obstacle opaque. One branch zeroed activations in 32 lattice cells, the other used a sham intervention; both went through the normal canonical reseal and cold-restore path. The first four recorded rows were identical. In one 40-decision seed, recurrent activations and some logits differed and decayed after intervention, but selected action slots did not differ. Thus these cells influenced network activity but the experiment did not show useful retained-target behavior. The object briefly reappeared later, and the temporary port was not a versioned production sensor. No accepted life was touched. Archive `a72acbcf5ce45cbba22d213c4a6589a46bbf19ecd96cd459e8d352f4cb783934`.

## Curiosity-gain tradeoff

In one occlusion scene the nearby obstacle acquired a high approach score. A fresh variant reduced the generic seen-object exploration edit from 80 to 8 without introducing an obstacle label; this was tested on seed 1846, not promoted. At 120 decisions, both variants consumed neutral food and avoided consuming the configured bitter item, but the lower-gain variant missed clear food while baseline ate it. At 240 decisions, baseline ate the hidden left-gap food and lower-gain did not; neither ate hidden right-gap food. Cold checks were exact. One seed cannot resolve generality, but bluntly reducing curiosity is not a demonstrated improvement. Archives: controls `ae69d456709d1aee61a58ba81e08495395d11e6d2180817191d57e805492b7cf`; mazes `b6c8750809faae6bace4856c08b9c9467a9b566efaf44a42ebdc4d77d77c70af`.

Temporary research changes were restored and the ordinary CPU example rebuilt. The research-only intervention and unversioned port require a distinct, validated ABI and CPU/GPU parity before any real use.

The adjacent `.patch` files are reproducibility records for these disposable experiments, not production-ready fixes. The intervention patch deliberately introduces an unversioned environment-controlled CPU input and a research intervention API; it must not be used with real saved lives or merged as a normal runtime change. The curiosity patch alters only the example's experimental weight recipe. Neither patch has GPU parity.
