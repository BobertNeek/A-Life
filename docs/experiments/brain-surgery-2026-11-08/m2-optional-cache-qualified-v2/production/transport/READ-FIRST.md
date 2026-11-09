# Source-patch transport only

The five numbered .txt pieces are opaque UTF-8 source text chunks. They are not independently applicable patches or runtime source. Do not add separators, reformat, change line endings or strip a final newline. Concatenate the raw bytes in numeric order (001 through 005) to reconstruct production/full-from-fbf2a517.patch. From this transport directory, for example:

    cat full-from-fbf2a517.patch.part-001.txt full-from-fbf2a517.patch.part-002.txt full-from-fbf2a517.patch.part-003.txt full-from-fbf2a517.patch.part-004.txt full-from-fbf2a517.patch.part-005.txt > ../full-from-fbf2a517.patch
    sha256sum ../full-from-fbf2a517.patch

The reconstructed file must be exactly 416,772 bytes with SHA256 9412792ee0b3ed9a1ac0f68bc83350a1721cd6db96df7415031731164a75ce67. Apply it only after verification and according to the main packet README. The original full patch and original manifests/hash list remain unchanged locally. In a published layout, the reconstructed full patch satisfies the existing production/full-from-fbf2a517.patch entry in SHA256SUMS.txt; the chunks are transport substitutes, not a revision of that manifest. No build, experiment or publication occurred during transport preparation.
