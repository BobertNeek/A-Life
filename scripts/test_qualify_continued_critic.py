import copy
import unittest
from qualify_continued_critic import check_slices, validate_admission


class SeparateSliceGates(unittest.TestCase):
    def fixture(self):
        return {key:dict(residual_mse=.02,out_of_support_predictions=0) for key in
                ["early16","whole_trajectory","last16"]}

    def test_initial_transient_fails_even_when_whole_average_passes(self):
        baseline=self.fixture(); observed=copy.deepcopy(baseline)
        observed["early16"]["residual_mse"]=1.687
        observed["whole_trajectory"]["residual_mse"]=.59
        with self.assertRaisesRegex(ValueError,"early16"):
            check_slices(baseline,observed)

    def test_nonfinite_or_support_escape_cannot_pass(self):
        for field,value in [("residual_mse",float("nan")),("out_of_support_predictions",1)]:
            baseline=self.fixture(); observed=copy.deepcopy(baseline)
            observed["last16"][field]=value
            with self.assertRaises(ValueError):check_slices(baseline,observed)

    def test_all_slices_inside_predeclared_limits_pass(self):
        check_slices(self.fixture(),self.fixture())

    def test_admission_must_be_read_only_and_bound_to_all_checkpoint_counters(self):
        cycle=dict(policy_version=7,actor_optimizer_step=12,value_optimizer_step=5012)
        valid=dict(cycle,neural_execution=False,weights_modified=False)
        validate_admission(valid,cycle)
        for key in ["policy_version","actor_optimizer_step","value_optimizer_step"]:
            changed=dict(valid);changed[key]+=1
            with self.assertRaises(ValueError):validate_admission(changed,cycle)
        for key in ["neural_execution","weights_modified"]:
            for bad in [True,None]:
                changed=dict(valid);changed[key]=bad
                with self.assertRaises(ValueError):validate_admission(changed,cycle)


if __name__=="__main__":unittest.main()
