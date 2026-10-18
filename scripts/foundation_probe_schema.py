"""Validate retained lesson-trace JSON against the native sensory contracts.

Sources: alife_core/{math,grounding,sensory_abi,perception}.rs and the app's
foundation_training/{lesson_trace,food_outcome}.rs. This validates evidence;
it does not normalize inputs or execute an organism.
"""

F32_MAX = 3.4028234663852886e38


class InvalidComparisonEvidence(ValueError):
    pass


def uint(value, bits, *, positive=False):
    return type(value) is int and int(positive) <= value < 2**bits


def invalid(label):
    raise InvalidComparisonEvidence(f"malformed or unsupported sensory input {label}")


def record(value, fields, label):
    if not isinstance(value, dict) or set(value) != set(fields):
        invalid(label)


def number(value, label, lower=-F32_MAX, upper=F32_MAX):
    # Bounds also reject NaN/infinity and integers that would overflow f32.
    if type(value) not in (int, float) or not lower <= value <= upper:
        invalid(label)


def integer(value, bits, label, *, positive=False):
    if not uint(value, bits, positive=positive):
        invalid(label)


def optional_id(value, label):
    if value is not None:
        integer(value, 64, label, positive=True)


def vector(value, length, label, lower=0.0, upper=1.0):
    if not isinstance(value, list) or len(value) != length:
        invalid(label)
    for element in value:
        number(element, label, lower, upper)


def position(value, label, *, quaternion=False):
    record(value, "xyzw" if quaternion else "xyz", label)
    for element in value.values():
        number(element, label)
    # Quatf::validate rejects zero, but does not require unit length.
    if quaternion and not any(value.values()):
        invalid(label)


def heard_tokens(tokens):
    if not isinstance(tokens, list) or len(tokens) != 16:
        invalid("heard_tokens")
    fields = {"utterance_id", "sequence_position", "source_kind", "speaker_id",
              "addressee", "source_entity", "token_id", "source_position",
              "confidence", "teacher_channel"}
    for token in tokens:
        if token is None:
            continue
        record(token, fields, "heard_tokens record")
        integer(token["utterance_id"], 64, "utterance_id", positive=True)
        integer(token["sequence_position"], 8, "sequence_position")
        if token["sequence_position"] >= 16:
            invalid("sequence_position")
        if token["source_kind"] not in ("Player", "Creature", "Teacher"):
            invalid("source_kind")
        for field in ("speaker_id", "addressee", "source_entity"):
            optional_id(token[field], field)
        integer(token["token_id"], 32, "token_id", positive=True)
        if token["token_id"] >= 256:
            invalid("token_id")
        position(token["source_position"], "source_position")
        number(token["confidence"], "heard confidence", 0, 1)
        if token["teacher_channel"] not in (None, "Hearing", "Vision", "Writing", "Gesture", "Object"):
            invalid("teacher_channel")


def semantic_context(context):
    # Option<SemanticContextRef>: absence of the private prior is native-valid.
    if context is None:
        return
    record(context, {"feature_flags", "confidence", "compressed_codes", "salience"}, "semantic_context")
    integer(context["feature_flags"], 32, "semantic feature_flags")
    number(context["confidence"], "semantic confidence", 0, 1)
    for field in ("compressed_codes", "salience"):
        if not isinstance(context[field], list):
            invalid("semantic " + field)
    # Native Vec fields have no fixed length or known-bit restriction.
    for code in context["compressed_codes"]:
        record(code, {"codebook_id", "code", "salience"}, "compressed code")
        integer(code["codebook_id"], 16, "codebook_id", positive=True)
        integer(code["code"], 32, "semantic code")
        number(code["salience"], "code salience", 0, 1)
    for entry in context["salience"]:
        record(entry, {"concept_id", "salience"}, "semantic salience")
        integer(entry["concept_id"], 64, "concept_id", positive=True)
        number(entry["salience"], "concept salience", 0, 1)


def object_slots(slots):
    if not isinstance(slots, list) or len(slots) > 16:
        invalid("object_slots")
    signed_vectors = {"bearing": 2, "relative_velocity": 3, "chemical": 3, "proprioception": 2}
    unit_vectors = {"color": 3, "material": 3, "shape": 3, "terrain": 2}
    fields = {"slot_index", "tracked_object_id", "distance", "contact", "temperature", "confidence"} | signed_vectors.keys() | unit_vectors.keys()
    for index, slot in enumerate(slots):
        record(slot, fields, "object slot")
        integer(slot["slot_index"], 16, "slot_index")
        if slot["slot_index"] != index:
            invalid("slot_index ordering")
        integer(slot["tracked_object_id"], 64, "tracked_object_id", positive=True)
        for field, length in signed_vectors.items():
            vector(slot[field], length, field, -1, 1)
        for field, length in unit_vectors.items():
            vector(slot[field], length, field)
        for field in ("distance", "contact", "confidence"):
            number(slot[field], field, 0, 1)
        number(slot["temperature"], "temperature", -1, 1)


def candidates(values, slots):
    if not isinstance(values, list) or not 1 <= len(values) <= 32:
        invalid("candidate support")
    fields = {"action", "family", "index", "logit", "observation", "target", "target_position"}
    families = ("Idle", "Rest", "Inspect", "Approach", "Avoid", "Contact", "Ingest", "Other")
    for index, candidate in enumerate(values):
        record(candidate, fields, "candidate schema")
        integer(candidate["index"], 16, "candidate index")
        if candidate["index"] != index:
            invalid("candidate index ordering")
        integer(candidate["action"], 32, "candidate action", positive=True)
        if candidate["family"] not in families:
            invalid("candidate family")
        observation = candidate["observation"]
        if observation != "None":
            record(observation, {"ObjectSlot"}, "candidate observation")
            integer(observation["ObjectSlot"], 16, "candidate observation slot")
            if observation["ObjectSlot"] >= len(slots):
                invalid("candidate observation slot reference")
        optional_id(candidate["target"], "candidate target")
        if candidate["target_position"] is not None:
            position(candidate["target_position"], "candidate target_position")
        number(candidate["logit"], "candidate logit")


def validate_sensory_context(context, candidate_values):
    position(context["position"], "position")
    position(context["gaze_rotation"], "gaze_rotation", quaternion=True)
    vector(context["terrain_ranges"], 16, "terrain_ranges")
    number(context["global_grip_before_decision"], "global_grip_before_decision", 0, 1)
    heard_tokens(context["heard_tokens"])
    semantic_context(context["semantic_context"])
    object_slots(context["object_slots"])
    candidates(candidate_values, context["object_slots"])
    integer(context["lesson_target"], 64, "lesson_target", positive=True)
    held = context["lesson_target_held_before_decision"]
    if held is not None and type(held) is not bool:
        invalid("lesson_target_held_before_decision")
    ownership = context["lesson_target_ownership_before_decision"]
    if ownership is None:
        if held is not None:
            invalid("missing target ownership")
    else:
        record(ownership, {"owner", "consumed"}, "target ownership")
        optional_id(ownership["owner"], "target owner")
        if type(ownership["consumed"]) is not bool:
            invalid("target consumed")
        if held != (ownership["owner"] == context["organism_id"] and not ownership["consumed"]):
            invalid("held target ownership coherence")
