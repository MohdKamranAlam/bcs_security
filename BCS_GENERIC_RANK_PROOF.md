# BCS Generic Rank Proof Attempt

## Family

We study the elliptic curve over the rational function field Q(t):

```text
E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4.
```

Equivalently, over `Q(t)`:

```text
E / Q(t): y^2 = x^3 + (17+t)x^2 + 5x + 4.
```

## Rational Section

The point

```text
P = (0, 2)
```

lies on every fiber, because substituting `x=0` gives

```text
y^2 = 4.
```

Thus `P=(0,2)` defines a rational section of the elliptic surface.

## Non-Torsion Argument

To prove that the generic Mordell-Weil rank over `Q(t)` is at least 1, it is
enough to show that `P` is not torsion over `Q(t)`.

Suppose for contradiction that `P` is torsion over `Q(t)`. Then there is some
integer `n > 0` such that

```text
nP = O
```

as an identity on the generic fiber. Specializing at any smooth fiber preserves
this identity, so the specialized point `P_t` would be torsion on every smooth
specialization.

At `t=66`, Sage verifies that

```text
E_66: y^2 = x^3 + 83x^2 + 5x + 4
```

has trivial torsion and exact rank 3. In particular, the point

```text
P_66 = (0, 2)
```

is non-torsion.

This contradicts the assumption that `P` was torsion over `Q(t)`.

Therefore `P=(0,2)` is a non-torsion section.

## Conclusion

The generic Mordell-Weil rank satisfies

```text
rank E(Q(t)) >= 1.
```

This is an exact structural result for the whole family, not just a numerical
observation about one specialization.

## Why This Matters

The family is not rank-positive by accident at isolated values. It has a
built-in non-torsion rational section. The higher ranks observed at `t=66` and
`t=1681` therefore occur on top of a genuine generic rank-at-least-one
structure.

