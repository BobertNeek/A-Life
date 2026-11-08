# V12 failed-retry count regression

Source: motor-reflex-retained-count-v12 commit 44f030b905fdece730f43403d78ef5734f8f3bd5, tree b3e99a273dd69964c217d69976a1700e9f286e8f; parent frozen V11 192ea624c839b409979da368c9faedcab54bd20c.

Only production changes remove the inconsistent failed-retry count ranges in runtime capture and world save validation. Required u8 wire field, schema2, all identity/pending/profile/sealed/evidence/error guards and live retry timing stay unchanged. Tests cover all256 valid counts and reject invalid evidence and malformed/missing counts. Preserve the prerepair count0 native failure.

Actual-worker allocation repeats the frozen V11 recipe/seed31118 once in fresh disposable state. One actual decision, two world intervals; genuine injected first rejection then real retry-only acknowledgment. Original90-second worker ownership/publication bounds and all authority/dirty/save/restore assertions remain. Additional assertions explicitly check count0 immediately after initial rejection and after the real old immutable publication is restored. No extra policy/world intervals, no autonomous cohort, no automatic cognitive trigger. Both old and new publications must come from actual workers; no snapshots or replacement publication may stand in.

Explicit cfg(test) SwiftShader Cpu/Vulkan constructor only; ordinary hardware admission remains unchanged. Record exact executable SHA before launch. Failure closes the allocation; preserve finite logs and diagnose before any later revision. No success claim until actual real-publication and restore assertions pass. Generated checkpoint files, assets, executable, and full arrays remain local and excluded from packages.

Separate follow-up coverage is still needed for genuine repeated failed retries through4 and restored-zero retry execution; the current fixture validates zero capture/restore and retry from the original matching live state, not a retry on the restored branch.
