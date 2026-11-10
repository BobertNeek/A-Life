#!/usr/bin/env bash
set -u
root="${ALIFE_CUE_ROOT:?Set ALIFE_CUE_ROOT to the study directory}"
binary="$root/cue-palette-executable"
test "$(sha256sum "$binary" | cut -d' ' -f1)" = "$(cut -d' ' -f1 "$root/cue-palette-matrix-binary.sha256")" || exit 1
count=0; limit=${1:-8}
for seed in 2026111554 2026111555; do
 for safe in red blue; do
  for side in left right; do
   for order in safe-first harm-first; do
    for palette in matched different; do
     for gain in baseline pain4; do
      for mode in normal sham; do
       name="$seed-$safe-$side-$order-$palette-$gain-$mode"; f="$root/cue-palette-matrix/$name.json"
       if [ -f "$f.failed" ]; then continue; fi
       if [ -f "$f" ]; then
        if python3 "$root/validate-cue-case.py" "$f" "$seed" "$safe" "$side" "$order" "$palette" "$gain" "$mode"; then continue; fi
        mv "$f" "$f.incomplete.$(date +%s%N)" || exit 1
       fi
       args=(-u ALIFE_RESEARCH_FAST_WRITE_SHAM -u ALIFE_RESEARCH_DECODER_TRACE_RESET -u ALIFE_RESEARCH_MEMORY_PAIN_GAIN "ALIFE_RESEARCH_APPROACH_SCALE=0.70" "ALIFE_RESEARCH_CUE_ORDER=$order" "ALIFE_RESEARCH_CUE_PALETTE=$palette")
       if [ "$gain" = pain4 ]; then args+=("ALIFE_RESEARCH_MEMORY_PAIN_GAIN=4.0"); fi
       if [ "$mode" = sham ]; then args+=("ALIFE_RESEARCH_FAST_WRITE_SHAM=1"); fi
       log="$root/cue-palette-matrix/$name.attempt-$(date +%s%N).log"
       env "${args[@]}" "$binary" "$f" "$seed" "$safe" "$side" > "$log" 2>&1
       status=$?
       if [ "$status" -eq 0 ] && python3 "$root/validate-cue-case.py" "$f" "$seed" "$safe" "$side" "$order" "$palette" "$gain" "$mode"; then valid=yes; else valid=no; fi
       printf '%s exit=%s valid=%s\n' "$name" "$status" "$valid" | tee -a "$root/cue-palette-matrix/resume-progress.log"
       if [ "$valid" = no ]; then touch "$f.failed"; fi
       count=$((count+1)); if [ "$count" -ge "$limit" ]; then exit 0; fi
      done
     done
    done
   done
  done
 done
done
