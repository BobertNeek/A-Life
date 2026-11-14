# Fresh anchored-V2 profile transfer

The anchored N2048 V2 graph can be freshly compiled with an explicit
`GroundedObjectSlotsV1`, `GroundedTerrainVisionV1`, or
`GroundedTerrainObjectPortsV2` profile. This does not change the default profile,
anchored topology, reference sensor anchors or existing SlotsV1 saves. The
signed-motor V3 recipe remains SlotsV1-only. Privileged-affordance input is not
admitted by this anchored constructor.

The profile is already part of the graph descriptor and foundation manifest
identity. Rebuilding a profile-bound asset is required; relabeling a saved asset
or acquired state is not a migration. An older reader rejects the newly supported
anchored/profile combinations. Full genetic pairs must be exported together.

To transfer fixed genetic coefficients, first compile the target profile's fresh
anchored graph. Verify every ordered synapse coordinate, route, decoder identity,
receptor and alpha against the source before copying weight bits. Verify dynamics,
receptors and old sensor assignments as well. Compile the exported full pair again.
A differing coordinate layout must not be silently remapped or treated as an
isomorphic weight transfer. New sensor assignments remain subject to the existing
anchored free-cell allocator and can fail if a future genotype exhausts it.

This is not necessarily a vision-only change. Existing terrain profiles activate
physical gaze/FOV/occlusion and their existing language/prior input ports when
hearing genes qualify. ObjectPortsV2 additionally supplies the four nearest visible
objects' existing48 neural lanes. No object-kind, nutrition, reward or target ID
is inserted as a new sense.

In the current headless adapter, SlotsV1 observation uses an identity sensing
rotation and range-only slot inclusion. Terrain profiles use body plus head yaw
and a ±110-degree sight cone with physical occlusion. Candidate bearing lanes0/1
are gaze-relative for terrain profiles, but relative-velocity XYZ remains in world
coordinates. Prepared-observer fixture placement therefore needs separate checks
for static bearing/distance, motion features and other sensory channels; it does
not make the entire feature vector observer-relative. Smell/touch remain separate
physical signals, so visual occlusion is not universal absence of information.

Terrain profiles already expose the seven body/head primitives and reserve only
three object candidates within the32-candidate budget. Legacy SlotsV1 with older
architecture does not have that same repertoire. Profile comparisons should use
matched architecture/head configurations, such as V3/head3 across all three
profiles, and still retain the distinct encoder/input semantics above.
