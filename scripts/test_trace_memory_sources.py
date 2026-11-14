import struct
import subprocess
import sys
from pathlib import Path
import unittest
from unittest.mock import patch
import trace_memory_sources as t


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def fixture(mid=1, object_id=1):
    f=[0.0]*83
    f[63]=f[66]=1.0
    r=dict(memory_id_raw=mid, tracked_object_id_raw=object_id,
           organism_id_raw=1, profile_id_raw=2, profile_schema_version=3,
           sensory_abi_version_raw=4, query_version_raw=5,
           query_feature_bits=list(map(bits,f)), last_tick_raw=1, family_raw=1,
           target_latent_bits=[bits(.1)], family_value_bits=[bits(0)]*4)
    bucket={k:r[k] for k in ('organism_id_raw','profile_id_raw','profile_schema_version','sensory_abi_version_raw','query_version_raw')}
    return f,r,dict(target_bucket=bucket,target_eligible=1,target_searched=1,target_matches=1)

class TraceTests(unittest.TestCase):
    def test_exact_f32_threshold(self):
        cutoff=bits(.72)
        self.assertFalse(t.passes_similarity(t.frombits(cutoff-1)))
        self.assertTrue(t.passes_similarity(t.frombits(cutoff)))
        self.assertTrue(t.passes_similarity(t.frombits(cutoff+1)))
        self.assertFalse(t.passes_similarity(t.f32(.55)))

    def test_same_object_suppresses_fallback_even_when_dissimilar(self):
        f,r,e=fixture(object_id=9)
        r['query_feature_bits']=[bits(0)]*83
        with self.assertRaisesRegex(ValueError,'same-identity'):
            t.reconstruct(f,[r],9,e)

    def test_absent_identity_eligible_distinct_from_matches(self):
        f,r,e=fixture()
        other=dict(r, memory_id_raw=2, tracked_object_id_raw=2)
        # Keep category, alter non-category sensory lanes to lower similarity.
        bad=list(other['query_feature_bits']); bad[0]=bits(1.0); other['query_feature_bits']=bad
        e.update(target_eligible=2,target_searched=2,target_matches=1)
        # Decouple category equivalence from controlled weighted score values.
        original=t.cosine
        def cosine(a,b,lo,hi):
            if lo==0 and b[0]==1.0:return 0.0
            return original(a,b,lo,hi)
        with patch.object(t,'cosine',side_effect=cosine):
            candidates,top=t.reconstruct(f,[r,other],9,e)
        self.assertEqual([x['memory_id'] for x in top],[1])
        self.assertEqual(len(candidates),1)

    def test_optimized_python_rejects_bad_receipt(self):
        code = "import trace_memory_sources as t; t.verify_result(0.0, [{'memory_id': 1}], {'target_latent': [1.0], 'best_target_source': 1})"
        proc = subprocess.run([sys.executable, '-O', '-c', code], capture_output=True, text=True, cwd=Path(__file__).resolve().parent)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn('hunger mismatch', proc.stderr)

    def test_match_count_is_post_top_four(self):
        f,r,e=fixture()
        records=[dict(r, memory_id_raw=i, tracked_object_id_raw=i) for i in range(1,7)]
        e.update(target_eligible=6,target_searched=6,target_matches=4)
        candidates,top=t.reconstruct(f,records,9,e)
        self.assertEqual(len(candidates),6)
        self.assertEqual([x['memory_id'] for x in top],[1,2,3,4])

    def test_rejects_truncated_bucket(self):
        f,r,e=fixture(); e['target_searched']=0
        with self.assertRaisesRegex(ValueError,'truncated'):
            t.reconstruct(f,[r],9,e)

if __name__=='__main__': unittest.main()
