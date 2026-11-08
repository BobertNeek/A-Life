# V2 executable preservation limitation

After the V2 life completed, the Cargo target was reused for V3 qualification.
The same-named test executable was replaced before a separate copy was made.
The original V2 executable bytes are therefore no longer preserved at the path
in build-receipt-v2.json. Its measured pre-exposure SHA-256 remains
32250e98824998e7186901e1ed892c3de496e4090fef7f2332659257ef6a26bd.

The complete executed source/708-input closure, old-asset equality check, plan,
qualification/runtime logs, exposure marker, actual diagnostic rows, outcome and
immutable build receipt remain intact. No life is rerun or reconstructed to
replace that evidence. Any later source rebuild would be labeled a rebuild, not
the original executable. This is a preservation error, not an outcome change.

V3 executable bytes are separately copied and hash-verified into its local
evidence directory before exposure. Those bytes remain excluded from source
publication, as do private runtime journals and state.
