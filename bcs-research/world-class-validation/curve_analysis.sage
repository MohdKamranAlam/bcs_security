#!/usr/bin/env sage
"""Optional Sage analysis (run: sage curve_analysis.sage).

Computes everything our pure-Python scripts cannot:
  * exact conductor (Tate's algorithm)
  * cremona label / LMFDB cross-reference
  * exact rank (2-descent + mwrank)
  * generators of E(Q)
  * exact L(E, 1), L'(E, 1)
  * matching weight-2 newform on Γ₀(N)
  * full BSD invariants  (regulator, real period, Tamagawa numbers)
"""
import json

E = EllipticCurve(QQ, [0, -2, 0, 5, 4])

result = {
    "curve":           "y^2 = x^3 - 2x^2 + 5x + 4",
    "discriminant":    int(E.discriminant()),
    "j_invariant":     str(E.j_invariant()),
    "conductor":       int(E.conductor()),
    "cremona_label":   str(E.cremona_label()),
    "torsion_order":   int(E.torsion_order()),
    "torsion_structure": str(E.torsion_subgroup().structure()),
    "rank":            int(E.rank()),
    "generators":      [str(g) for g in E.gens()],
    "tamagawa_product": int(E.tamagawa_product()),
    "real_period":     float(E.period_lattice().real_period()),
    "L_at_1":          float(E.lseries().at1()[0]),
    "regulator":       float(E.regulator()),
    "modular_form_q_expansion": str(E.modular_form().q_expansion(20)),
}

with open("sage_results.json", "w") as f:
    json.dump(result, f, indent=2)

for k, v in result.items():
    print(f"{k:<28} : {v}")
