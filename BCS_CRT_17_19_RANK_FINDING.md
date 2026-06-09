# BCS CRT 17/19 Rank Finding

We study the family:

E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4.

Main result:

For every integer t satisfying

t = 66 mod 323,

we get the finite-field point-count exchange:

#E_t(F_17) = 19
#E_t(F_19) = 17

This happens because:

t = -2 mod 17 gives #E_t(F_17) = 19
t = 9 mod 19 gives #E_t(F_19) = 17

Combining these two congruences gives:

t = 66 mod 323.

Sage rank evidence for tested values:

t=-257: rank 1
t=66: rank 3
t=389: rank 2
t=712: rank 2
t=1035: rank 1
t=1358: rank 2
t=1681: rank 3

Strongest examples:

E_66 has rank 3.
E_1681 has rank 3.

Caveat:

The point-count exchange is exact. The rank pattern is computational evidence from tested cases.
