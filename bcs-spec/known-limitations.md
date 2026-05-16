# BCS Known Limitations

## Honest security boundary

BCS-256 is a research candidate for classical elliptic-curve cryptography. It must not be described as unbreakable, fully proven, standardized, or post-quantum by itself.

## Current open items

1. **Academic primality certificates**
   - SymPy BPSW evidence is strong for engineering use.
   - A white paper should include Sage/PARI/Magma proof certificates for `p` and `n`.

2. **Independent cardinality proof**
   - `#E(F_p) = n` should be independently verified in Sage/Magma.

3. **Exact embedding degree**
   - The Python audit proves a lower bound.
   - Exact `k = ord_n(p)` should be computed once the relevant factorization is complete.

4. **Twist security**
   - The twist has a large composite factor `Q2` not fully factored in the current audit.
   - Until complete twist factorization is available, implementations must enforce strict point validation.

5. **Side channels**
   - Python reference code is not constant-time.
   - Production code must use constant-time scalar multiplication, no secret-dependent branches, and no secret-dependent memory access.

6. **Quantum security**
   - BCS ECC is broken by Shor's algorithm on a fault-tolerant quantum computer.
   - Post-quantum deployments must use a hybrid construction such as `BCS-521 ECDH + ML-KEM-1024`.

## Recommended public wording

> BCS-256 is a Qur'an-inspired, rigidly generated, verified 256-bit prime-order elliptic-curve research candidate for classical ECC, secure only with strict point validation and not post-quantum without ML-KEM hybridization.
