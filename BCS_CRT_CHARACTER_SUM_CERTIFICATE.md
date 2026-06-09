# BCS CRT 17/19 Character-Sum Certificate

Family:

```text
E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4
```

For odd prime `p`, the finite-field point count is

```text
#E_t(F_p) = p + 1 + sum_x chi(x^3 + (17+t)x^2 + 5x + 4)
```

where `chi` is the quadratic character modulo `p`.

## Residue Scan

- residues mod 17 with `#E_t(F_17)=19`: `[15]`
- residues mod 19 with `#E_t(F_19)=17`: `[9]`

Thus the relevant congruences are:

```text
t = 15 = -2 mod 17
t = 9 mod 19
```

By CRT this is equivalent to:

```text
t = 66 mod 323
```

## Certificate for p=17

- prime p: `17`
- residue t mod p: `15`
- character sum S_p(t): `1`
- point count p + 1 + S_p(t): `17 + 1 + (1) = 19`

| x | f_t(x) mod p | chi(f_t(x)) |
|---:|---:|---:|
| 0 | 4 | 1 |
| 1 | 8 | 1 |
| 2 | 14 | -1 |
| 3 | 11 | -1 |
| 4 | 5 | -1 |
| 5 | 2 | 1 |
| 6 | 8 | 1 |
| 7 | 12 | -1 |
| 8 | 3 | -1 |
| 9 | 4 | 1 |
| 10 | 4 | 1 |
| 11 | 9 | 1 |
| 12 | 8 | 1 |
| 13 | 7 | -1 |
| 14 | 12 | -1 |
| 15 | 12 | -1 |
| 16 | 13 | 1 |

## Certificate for p=19

- prime p: `19`
- residue t mod p: `9`
- character sum S_p(t): `-3`
- point count p + 1 + S_p(t): `19 + 1 + (-3) = 17`

| x | f_t(x) mod p | chi(f_t(x)) |
|---:|---:|---:|
| 0 | 4 | 1 |
| 1 | 17 | 1 |
| 2 | 12 | -1 |
| 3 | 14 | -1 |
| 4 | 10 | -1 |
| 5 | 6 | 1 |
| 6 | 8 | -1 |
| 7 | 3 | -1 |
| 8 | 16 | 1 |
| 9 | 15 | -1 |
| 10 | 6 | 1 |
| 11 | 14 | -1 |
| 12 | 7 | 1 |
| 13 | 10 | -1 |
| 14 | 10 | -1 |
| 15 | 13 | -1 |
| 16 | 6 | 1 |
| 17 | 14 | -1 |
| 18 | 5 | 1 |
