# BCS-521 Sage independent verification script
#
# Run options:
#   A) Local SageMath:
#        sage bcs521_sage_proof.sage
#   B) Google Colab with SageMath kernel:
#        Open BCS_521_SAGE_COLAB.ipynb (see README)
#        OR copy this file's body into a SageMath Colab cell.
#
# Purpose:
#   Final-grade independent proof of BCS-521 parameters using Sage's
#   own number theory (independent of the Rust + PARI/GP search).
#
# Checks:
#   1. p is prime (with proof=True ECPP/Pocklington certificate path)
#   2. n is prime (with proof=True)
#   3. #E(F_p) = n via Sage's own SEA algorithm
#   4. Generator G = (0, 2) lies on E
#   5. n * G = O (point of order n)
#   6. cofactor h = 1
#   7. Hasse bound holds
#   8. Curve is not anomalous
#   9. Exact embedding degree k = ord_n(p)
#  10. MOV threshold k >= 100 (very strong)
#  11. Twist order partial factorization for cofactor policy

from sage.all import *
import json
from datetime import datetime, timezone

# BCS-521 parameters (frozen 2026-05-16)
p = Integer(6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363)
n = Integer(6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231)

F = GF(p, proof=True)
# E: y^2 = x^3 - 2*x^2 + 5*x + 4  in Weierstrass [a1,a2,a3,a4,a6]
E = EllipticCurve(F, [0, -2, 0, 5, 4])
G = E(0, 2)

def status(name, ok, evidence=""):
    mark = "PASS" if ok else "FAIL"
    print(f"[{mark}] {name}")
    if evidence:
        print(f"       {evidence}")
    return {"name": name, "ok": bool(ok), "evidence": str(evidence)[:500]}

print("=" * 72)
print("BCS-521 Sage independent proof")
print(datetime.now(timezone.utc).isoformat())
print("=" * 72)
print(f"p bits = {p.nbits()}")
print(f"n bits = {n.nbits()}")
print()

results = []

# ----- (1) p prime -----
print(">>> Checking p is prime (proof=True). May take 30-120 s for 521-bit ECPP.")
results.append(status("p is prime, proof=True", p.is_prime(proof=True)))

# ----- (2) n prime -----
print("\n>>> Checking n is prime (proof=True).")
results.append(status("n is prime, proof=True", n.is_prime(proof=True)))

# ----- (3) Independent cardinality -----
print("\n>>> Computing #E(F_p) via Sage's SEA. May take 5-15 min for 521-bit.")
card = E.cardinality(proof=True)
results.append(status("E.cardinality(proof=True) == n", card == n, f"card = {card}"))

# ----- (4) G on curve -----
results.append(status("G = (0,2) lies on E", G in E))

# ----- (5) n * G = O -----
print("\n>>> Checking n * G = O.")
results.append(status("n * G is infinity", (n * G).is_zero()))

# ----- (6) cofactor -----
results.append(status("cofactor h = #E / n = 1", card // n == 1 and card % n == 0))

# ----- (7) Hasse -----
trace = p + 1 - n
results.append(status("Hasse bound |t| <= 2*sqrt(p)", trace**2 <= 4*p, f"trace = {trace}"))

# ----- (8) Anomalous -----
results.append(status("not anomalous (p != n)", p != n))

# ----- (9) Embedding degree -----
print("\n>>> Computing exact embedding degree k = ord_n(p). Fast for prime n.")
k = Mod(p, n).multiplicative_order()
results.append(status("exact embedding degree k computed", k > 0, f"k = {k}"))

# ----- (10) MOV threshold -----
results.append(status("MOV threshold k >= 100", k >= 100, f"k = {k}"))
results.append(status("MOV threshold k >= 2^40 (very strong)", k >= 2**40))

# ----- (11) Twist analysis -----
print("\n>>> Twist analysis.")
twist_order = 2 * (p + 1) - n
print(f"twist_order bits = {twist_order.nbits()}")

# Partial factorization with bounded effort (avoid hanging)
print("Trying small-factor trial division up to 10^6 ...")
small_factors = {}
remainder = twist_order
for q in primes(10**6):
    while remainder % q == 0:
        small_factors[int(q)] = small_factors.get(int(q), 0) + 1
        remainder //= q
    if remainder < q*q:
        break

print(f"small factors found: {small_factors}")
print(f"remaining unfactored part bits = {remainder.nbits()}")
remaining_is_prp = remainder.is_pseudoprime() if remainder > 1 else True
print(f"remaining part is probable prime: {remaining_is_prp}")

results.append(status("twist small factors enumerated <= 10^6", True, str(small_factors)))
results.append(status("twist large cofactor is PRP", remaining_is_prp,
                      f"largest part bits = {remainder.nbits()}"))

# Cofactor policy assessment
max_small = max(small_factors.keys()) if small_factors else 1
results.append(status("largest small twist factor < 2^32", max_small < 2**32,
                      f"max small = {max_small}"))

out = {
    "timestamp_utc": datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%SZ"),
    "curve_name": "BCS-521",
    "curve_equation": "y^2 = x^3 - 2x^2 + 5x + 4",
    "p": str(p),
    "n": str(n),
    "trace_of_frobenius": str(trace),
    "cardinality": str(card),
    "embedding_degree": str(k),
    "twist_order": str(twist_order),
    "twist_small_factors": {str(k_): v for k_, v in small_factors.items()},
    "twist_remaining_part_bits": int(remainder.nbits()),
    "twist_remaining_is_prp": bool(remaining_is_prp),
    "generator": {"Gx": str(G[0]), "Gy": str(G[1])},
    "cofactor_h": int(card // n),
    "ECDLP_security_bits_estimate": int(n.nbits() // 2),
    "results": results,
}

fname = "bcs521_sage_proof_result.json"
with open(fname, "w") as f:
    json.dump(out, f, indent=2)
print(f"\nSaved {fname}")

# Final summary
print("\n" + "=" * 72)
print("FINAL VERDICT")
print("=" * 72)
all_ok = all(r["ok"] for r in results)
critical_ok = all(r["ok"] for r in results if r["name"] in {
    "p is prime, proof=True",
    "n is prime, proof=True",
    "E.cardinality(proof=True) == n",
    "G = (0,2) lies on E",
    "n * G is infinity",
    "cofactor h = #E / n = 1",
    "Hasse bound |t| <= 2*sqrt(p)",
    "not anomalous (p != n)",
})
print(f"All checks passed: {all_ok}")
print(f"Critical checks passed: {critical_ok}")
print("Verdict: " + ("CONDITIONAL PASS (twist needs validation policy)" if critical_ok else "FAIL"))
