# BCS-256 Sage verification script
# Run with:
#   sage bcs256_sage_proof.sage
#
# Purpose:
#   Independent final-grade verification of p, n, cardinality, embedding degree,
#   and twist-status for the BCS-256 research curve.

from sage.all import *
import json
from datetime import datetime, timezone

p = Integer(75403776646910504885013085564245979049841362888363155420739536990720881516533)
n = Integer(75403776646910504885013085564245979049566799732270309665248923838363814402301)

F = GF(p, proof=True)
E = EllipticCurve(F, [0, -2, 0, 5, 4])
G = E(0, 2)

def status(name, ok, evidence=""):
    mark = "PASS" if ok else "FAIL"
    print(f"[{mark}] {name}")
    if evidence:
        print(f"       {evidence}")
    return {"name": name, "ok": bool(ok), "evidence": str(evidence)}

print("=" * 72)
print("BCS-256 Sage proof")
print(datetime.now(timezone.utc).isoformat())
print("=" * 72)

results = []

results.append(status("p is prime, proof=True", p.is_prime(proof=True)))
results.append(status("n is prime, proof=True", n.is_prime(proof=True)))

card = E.cardinality(proof=True)
results.append(status("E.cardinality(proof=True) == n", card == n, f"cardinality = {card}"))
results.append(status("G lies on E", G in E, f"G = {G}"))
results.append(status("n*G is infinity", (n * G).is_zero()))
results.append(status("cofactor h = #E/n = 1", card // n == 1 and card % n == 0))

trace = p + 1 - n
results.append(status("Hasse bound", trace^2 <= 4*p, f"trace = {trace}"))
results.append(status("not anomalous", p != n))

print("\nComputing exact embedding degree k = ord_n(p) ...")
k = Mod(p, n).multiplicative_order()
results.append(status("exact embedding degree k computed", k > 0, f"k = {k}"))
results.append(status("MOV threshold k >= 20", k >= 20))

print("\nTwist analysis ...")
twist_order = 2*(p + 1) - n
print(f"twist_order = {twist_order}")
print("Attempting factorization. This may take long for Q2...")
try:
    twist_factor = factor(twist_order)
    print(f"twist factorization = {twist_factor}")
    largest_prime_power = max([q^e for q, e in twist_factor])
    results.append(status("twist factorization completed", True, str(twist_factor)))
    results.append(status("largest twist prime-power >= 2^200", largest_prime_power >= 2^200, f"largest = {largest_prime_power}"))
except Exception as exc:
    results.append(status("twist factorization completed", False, repr(exc)))
    print("Formal policy: until full twist factorization is available, strict point validation is mandatory.")

out = {
    "timestamp_utc": datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%SZ"),
    "p": str(p),
    "n": str(n),
    "trace": str(trace),
    "cardinality": str(card),
    "embedding_degree": str(k),
    "twist_order": str(twist_order),
    "results": results,
}

name = "bcs256_sage_proof_result.json"
with open(name, "w") as f:
    json.dump(out, f, indent=2)
print(f"\nSaved {name}")
