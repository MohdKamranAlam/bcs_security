# BCS Rank-3 Mini Report

## Family

```text
E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4.
```

The CRT class

```text
t = 66 mod 323
```

satisfies the exact local point-count exchange:

```text
#E_t(F_17) = 19
#E_t(F_19) = 17
```

This report focuses on the two strongest tested rank-3 examples.

## Example 1: t = 66

Curve:

```text
E_66: y^2 = x^3 + 83x^2 + 5x + 4.
```

Arithmetic data:

| invariant | value |
|---|---:|
| discriminant | `-143158704` |
| discriminant factorization | `-2^4 * 3 * 13 * 61 * 3761` |
| conductor | `71579352` |
| conductor factorization | `2^3 * 3 * 13 * 61 * 3761` |
| torsion order | `1` |
| rank bounds | `(3, 3)` |
| algebraic rank | `3` |
| analytic rank | `3` |
| `#E_66(F_17)` | `19` |
| `#E_66(F_19)` | `17` |

Generators found by Sage:

```text
(-1, 9)
(0, 2)
(-45, 277)
```

## Example 2: t = 1681

Curve:

```text
E_1681: y^2 = x^3 + 1698x^2 + 5x + 4.
```

Arithmetic data:

| invariant | value |
|---|---:|
| discriminant | `-1252131133184` |
| bad primes | `2;4891137239` |
| conductor | `78258195824` |
| rank bounds | `(3, 3)` |
| algebraic rank | `3` |
| analytic rank | `3` |
| `#E_1681(F_17)` | `19` |
| `#E_1681(F_19)` | `17` |

Generators found by Sage:

```text
(0, 2)
(153/4, 12751/8)
(12/121, 6128/1331)
```

## Interpretation

The local 17/19 exchange is exact for the full CRT class `t = 66 mod 323`.
The rank-3 examples show that this same class contains globally rich
Mordell-Weil behavior.

The safest current claim is:

```text
The CRT class t = 66 mod 323 gives an exact 17/19 finite-field exchange and
contains certified rank-3 specializations at t=66 and t=1681.
```

## Caveat

The point-count exchange is a theorem. The broader claim that the whole CRT
class has unusually high rank is not yet proven; it is supported by the tested
Sage data.


## Selected Rank-3 Verification CSV

The selected Sage verification file `bcs_crt_selected_rank.csv` confirms:

| t | rank bounds | rank | analytic rank | #F17 | #F19 |
|---:|---:|---:|---:|---:|---:|
| 66 | (3,3) | 3 | 3 | 19 | 17 |
| 1681 | (3,3) | 3 | 3 | 19 | 17 |

This independently confirms that both strongest CRT-class examples are exact rank-3 curves with matching analytic rank.

## Selected Rank-3 Verification CSV

The selected Sage verification file `bcs_crt_selected_rank.csv` confirms:

| t | rank bounds | rank | analytic rank | #F17 | #F19 |
|---:|---:|---:|---:|---:|---:|
| 66 | (3,3) | 3 | 3 | 19 | 17 |
| 1681 | (3,3) | 3 | 3 | 19 | 17 |

This independently confirms that both strongest CRT-class examples are exact rank-3 curves with matching analytic rank.
