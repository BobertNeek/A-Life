# Harmful-food re-acquisitions after the first harmful meal

Offline descriptive analysis only; no new life, exposure, replay, native execution, or source change.

## Results

- s1_old_world: first harmful meal tick/sequence 4/4; 7 later actual H acquisitions, ticks 48, 73, 83, 124, 127, 134, 165. Counts: {'h_reacquisitions': 7, 'grab_reacquisitions': 7, 'eat_preparation_reacquisitions': 0, 'later_h_eat_selections': 0, 'later_h_meals': 0, 'held_before_decisions': 103}.
- s1_new_world: first harmful meal tick/sequence 4/4; 12 later actual H acquisitions, ticks 48, 73, 83, 124, 127, 134, 165, 175, 180, 220, 232, 248. Counts: {'h_reacquisitions': 12, 'grab_reacquisitions': 12, 'eat_preparation_reacquisitions': 0, 'later_h_eat_selections': 0, 'later_h_meals': 0, 'held_before_decisions': 46}.
- s2_new_world: first harmful meal tick/sequence 17/17; 8 later actual H acquisitions, ticks 105, 128, 144, 174, 195, 216, 228, 254. Counts: {'h_reacquisitions': 8, 'grab_reacquisitions': 8, 'eat_preparation_reacquisitions': 0, 'later_h_eat_selections': 0, 'later_h_meals': 0, 'held_before_decisions': 53}.
- s2_old_world: first harmful meal tick/sequence 36/36; 5 later actual H acquisitions, ticks 91, 116, 128, 218, 228. Counts: {'h_reacquisitions': 5, 'grab_reacquisitions': 4, 'eat_preparation_reacquisitions': 1, 'later_h_eat_selections': 1, 'later_h_meals': 0, 'held_before_decisions': 117}.

## Exact recall fields at each H acquisition

All source and confidence numbers below are copied without rounding. Full candidates for H and S, source-decision context, ownership and receipts are in analysis.json. Each row contrasts the chosen candidate with concurrent H/EAT recall. Source sequence refers only to best FAMILY source; target-source identities and target valence are not exported. Family valence is an aggregate family-value channel, not necessarily the valence of the single best source.

### s1_old_world

- Tick/sequence 48/48: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9486563801765442, "family_sources": 0, "family_confidence": 0.0, "family_valence": 0.0, "source_sequence": null, "source_exact_ingest": false, "source_classification": "no_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9486563801765442, "family_sources": 1, "family_confidence": 0.9270134568214417, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 73/73: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9147512316703796, "family_sources": 1, "family_confidence": 0.9258044362068176, "family_valence": -0.00018369060126133263, "source_sequence": 55, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9147512316703796, "family_sources": 1, "family_confidence": 0.8965010643005371, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 83/83: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9530885219573975, "family_sources": 2, "family_confidence": 0.9593082070350647, "family_valence": -0.00021177256712689996, "source_sequence": 76, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9530885219573975, "family_sources": 1, "family_confidence": 0.9228436350822449, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 124/124: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9986146688461304, "family_sources": 2, "family_confidence": 0.9580396413803101, "family_valence": -0.00023674374097026885, "source_sequence": 73, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9986146688461304, "family_sources": 1, "family_confidence": 0.8926664590835571, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 127/127: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9168057441711426, "family_sources": 3, "family_confidence": 0.884959876537323, "family_valence": -0.0002676669682841748, "source_sequence": 126, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9168057441711426, "family_sources": 1, "family_confidence": 0.7963053584098816, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 134/134: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9509969353675842, "family_sources": 3, "family_confidence": 0.8846139907836914, "family_valence": -0.0002635275886859745, "source_sequence": 129, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9509969353675842, "family_sources": 1, "family_confidence": 0.795753002166748, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 165/165: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9974523782730103, "family_sources": 3, "family_confidence": 0.9184577465057373, "family_valence": -0.0002123768354067579, "source_sequence": 124, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9974523782730103, "family_sources": 1, "family_confidence": 0.8898430466651917, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
### s1_new_world

- Tick/sequence 48/48: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9486563801765442, "family_sources": 0, "family_confidence": 0.0, "family_valence": 0.0, "source_sequence": null, "source_exact_ingest": false, "source_classification": "no_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9486563801765442, "family_sources": 1, "family_confidence": 0.9270134568214417, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 73/73: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9147512316703796, "family_sources": 1, "family_confidence": 0.9258044362068176, "family_valence": -0.00018369060126133263, "source_sequence": 55, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9147512316703796, "family_sources": 1, "family_confidence": 0.8965010643005371, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 83/83: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9530885219573975, "family_sources": 2, "family_confidence": 0.9593082070350647, "family_valence": -0.00021177256712689996, "source_sequence": 76, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9530885219573975, "family_sources": 1, "family_confidence": 0.9228436350822449, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 124/124: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9986146688461304, "family_sources": 2, "family_confidence": 0.9580396413803101, "family_valence": -0.00023674374097026885, "source_sequence": 73, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9986146688461304, "family_sources": 1, "family_confidence": 0.8926664590835571, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 127/127: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9168057441711426, "family_sources": 3, "family_confidence": 0.884959876537323, "family_valence": -0.0002676669682841748, "source_sequence": 126, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9168057441711426, "family_sources": 1, "family_confidence": 0.7963053584098816, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 134/134: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9509969353675842, "family_sources": 3, "family_confidence": 0.8846139907836914, "family_valence": -0.0002635275886859745, "source_sequence": 129, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9509969353675842, "family_sources": 1, "family_confidence": 0.795753002166748, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 165/165: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9974523782730103, "family_sources": 3, "family_confidence": 0.9184577465057373, "family_valence": -0.0002123768354067579, "source_sequence": 124, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9974523782730103, "family_sources": 1, "family_confidence": 0.8898430466651917, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 175/175: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9451364278793335, "family_sources": 3, "family_confidence": 0.9081283211708069, "family_valence": -0.00021303573157638311, "source_sequence": 84, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9451364278793335, "family_sources": 1, "family_confidence": 0.9170755743980408, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 180/180: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.948062002658844, "family_sources": 3, "family_confidence": 0.9079342484474182, "family_valence": -0.00021303939865902066, "source_sequence": 84, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.948062002658844, "family_sources": 1, "family_confidence": 0.9168655276298523, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 220/220: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9970757961273193, "family_sources": 3, "family_confidence": 0.9151942729949951, "family_valence": -0.00021237802866380662, "source_sequence": 124, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9970757961273193, "family_sources": 1, "family_confidence": 0.8876700401306152, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 232/232: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9420028924942017, "family_sources": 3, "family_confidence": 0.8798645734786987, "family_valence": -0.00020569024491123855, "source_sequence": 134, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9420028924942017, "family_sources": 1, "family_confidence": 0.7919027805328369, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 248/248: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9444773197174072, "family_sources": 3, "family_confidence": 0.8806775808334351, "family_valence": -0.00021621640189550817, "source_sequence": 240, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9444773197174072, "family_sources": 1, "family_confidence": 0.7914310097694397, "family_valence": -0.2908289134502411, "source_sequence": 4, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
### s2_new_world

- Tick/sequence 105/105: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9390430450439453, "family_sources": 0, "family_confidence": 0.0, "family_valence": 0.0, "source_sequence": null, "source_exact_ingest": false, "source_classification": "no_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9390430450439453, "family_sources": 1, "family_confidence": 0.9313511848449707, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 128/128: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9327830672264099, "family_sources": 1, "family_confidence": 0.8159627318382263, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9327830672264099, "family_sources": 1, "family_confidence": 0.8060214519500732, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 144/144: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9401348233222961, "family_sources": 1, "family_confidence": 0.9370497465133667, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9401348233222961, "family_sources": 1, "family_confidence": 0.9251490831375122, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 174/174: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9464585781097412, "family_sources": 1, "family_confidence": 0.9383313655853271, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9464585781097412, "family_sources": 1, "family_confidence": 0.9267538189888, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 195/195: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9450592994689941, "family_sources": 1, "family_confidence": 0.9371830224990845, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9450592994689941, "family_sources": 1, "family_confidence": 0.9256954789161682, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 216/216: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9434499144554138, "family_sources": 1, "family_confidence": 0.9356675744056702, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9434499144554138, "family_sources": 1, "family_confidence": 0.9241658449172974, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 228/228: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9444250464439392, "family_sources": 1, "family_confidence": 0.9349774718284607, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9444250464439392, "family_sources": 1, "family_confidence": 0.9234572649002075, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 254/254: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9202775955200195, "family_sources": 1, "family_confidence": 0.8091163635253906, "family_valence": -0.000227776647079736, "source_sequence": 105, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9202775955200195, "family_sources": 1, "family_confidence": 0.7991440892219543, "family_valence": -0.2909366488456726, "source_sequence": 17, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
### s2_old_world

- Tick/sequence 91/91: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9376962184906006, "family_sources": 0, "family_confidence": 0.0, "family_valence": 0.0, "source_sequence": null, "source_exact_ingest": false, "source_classification": "no_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9376962184906006, "family_sources": 1, "family_confidence": 0.9339603185653687, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 116/116: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9435269236564636, "family_sources": 2, "family_confidence": 0.8806429505348206, "family_valence": -0.00017851145821623504, "source_sequence": 97, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9435269236564636, "family_sources": 1, "family_confidence": 0.8084234595298767, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 128/128: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.943453848361969, "family_sources": 2, "family_confidence": 0.8844310641288757, "family_valence": -0.0001813883427530527, "source_sequence": 116, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.943453848361969, "family_sources": 1, "family_confidence": 0.8078555464744568, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 218/218: primitive 211, family 5 (Contact), event acquired, phase completed.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9342831969261169, "family_sources": 2, "family_confidence": 0.8775427937507629, "family_valence": -0.00018130481475964189, "source_sequence": 116, "source_exact_ingest": false, "source_classification": "non_exact_ingestion_best_family_source"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9342831969261169, "family_sources": 1, "family_confidence": 0.8024718165397644, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
- Tick/sequence 228/228: primitive 210, family 6 (Ingest), event acquired, phase preparation.
  - Chosen: {"target_sources": 4, "target_confidence": 0.9424678087234497, "family_sources": 1, "family_confidence": 0.8026189804077148, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}
  - Concurrent H/EAT: {"target_sources": 4, "target_confidence": 0.9424678087234497, "family_sources": 1, "family_confidence": 0.8026189804077148, "family_valence": -0.29094424843788147, "source_sequence": 36, "source_exact_ingest": true, "source_classification": "exact_harmful_ingestion"}

## Interpretation and limits

- Acquisition requires an actual receipt transfer of H from nobody to self. An EAT selection, failed attempt, or continuing ownership alone is not counted as a new acquisition. EAT preparation acquires without consuming. All 32 reacquisitions have event acquired, not atomic_swap; the reverse S-to-H swap in the new seed-2 case occurs before its first harmful meal.
- 31 reacquisitions use GRAB/Contact, and one uses EAT/Ingest preparation. Contact best-family sources are not exact ingestion evidence; their source-decision receipts are included, without pretending that a multichannel decision uniquely identifies an unexported memory episode. A non-null source sequence alone does not prove harmful ingestion.
- The exact-ingestion flag binds the best family source to EAT + Ingest + Consumed + the same tracked target. Coupled with the validated prior physical meal and sequence it identifies the harmful ingestion specifically. An exact safe-food ingestion recall is not harmful-food recall.
- Observed negative H/Ingest recall alongside choosing H/Contact motivates inspecting cross-family use of outcome evidence. It does not establish missing retrieval, learned avoidance, causal credit assignment, or why the action was chosen. A selected EAT preparation with negative exact harm recall is likewise descriptive, not proof of a brain defect.
- Only two seeds and a bundled world change. Additional reacquisitions are not by themselves worse behavior: opportunities and release/holding duration differ. These data contain no candidate action probabilities, target-source identities, neural activations or ablations. No hidden-state or causal-learning inference is supported.
- All public-file manifest and SHA256SUMS hashes passed, all closed CASE schemas and row receipt semantics passed, source bindings match AS-RUN, and metrics exactly recompute. Original native summary bytes are outside this permitted copy; their previous verification is recorded by checks/AS-RUN, not independently repeated here.
- The requested active-v5/receipt_projection.rs path does not exist. The identically hashed projection file in both frozen source repos is used instead: crates/alife_training/examples/support/world_swap_receipt_projection.rs (e1a4a48ad800c3290ac8e046b427d47099ae65812bfc4ed0d75a82908330061c), especially lines 245–261. report_contract.py labels itself reconstructed, not byte-equivalent to an earlier inaccessible original. No equivalence to that original is claimed.

## Reproduction

Run python -B /workspace/scratch/ee5e08a3ca9b/harmful-food-regrabs-analysis-v1/analyze.py. This only reads the approved public/scalar/source inputs and writes this analysis directory. Exact input paths and hashes are recorded in analysis.json.
