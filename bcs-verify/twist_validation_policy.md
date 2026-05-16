# BCS Twist Validation Policy

## Current status

For BCS-256, the main group has prime order `n`, but the quadratic twist order contains a large composite factor `Q2` whose full factorization is not yet completed in the Python audit.

Until full twist factorization proves a sufficiently large prime factor, every BCS implementation must use validation-only twist defense.

## Mandatory validation rules

A BCS implementation must reject every received public point unless all checks pass:

```text
1. Point is not infinity.
2. x and y are canonical field elements: 0 <= x < p and 0 <= y < p.
3. y^2 mod p == x^3 - 2x^2 + 5x + 4 mod p.
4. Scalar multiplication result is not infinity.
```

Because the BCS-256 main group has prime order and cofactor 1, any point passing the curve equation check is in the intended group.

## Forbidden behavior

- Do not multiply a secret scalar by an unvalidated point.
- Do not accept compressed points unless decompression includes the same validation.
- Do not expose different error timing based on secret values.
- Do not log peer public keys in production unless explicitly required and privacy-reviewed.
- Never log private keys or raw shared secrets.

## Review condition

If Q2 is later fully factored and the twist has a large enough prime subgroup, this policy can be relaxed from mandatory defense to defense-in-depth. Until then, it is part of the BCS-256 security claim.
