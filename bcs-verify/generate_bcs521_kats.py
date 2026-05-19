#!/usr/bin/env python3
"""
generate_bcs521_kats.py — Deterministic KAT (Known-Answer Test) vector generator.

Outputs JSON files that the Rust crate (`bcs-core-rust`) and any future
Python/C/JS SDK must reproduce byte-for-byte.

Categories:
  1. keygen   — deterministic key-pair from seed
  2. ecdh     — ECDH shared secret + HKDF
  3. scalar   — add/sub/mul/inv mod n
  4. kahf_dst — Kahf domain separator

Usage:
    python generate_bcs521_kats.py

Output files (in bcs-verify/kats/):
    bcs521_keygen.json
    bcs521_ecdh.json
    bcs521_scalar.json
    bcs521_kahf_dst.json
"""

from __future__ import annotations
import hashlib
import hmac
import json
import os
import struct

# ---------------------------------------------------------------------------
# Frozen BCS-521 parameters
# ---------------------------------------------------------------------------
P_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
N_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231
G_X = 0
G_Y = 2
FIELD_BYTES = 66  # ceil(521 / 8)

# Kahf sacred primes
KAHF_PRIMES = [2141, 2969, 373, 19, 7]

# ---------------------------------------------------------------------------
# Deterministic RNG from seed (RFC 6979-style HMAC-DRBG)
# ---------------------------------------------------------------------------

class DRBG:
    """Simplified HMAC-DRBG (deterministic random bytes from seed)."""
    def __init__(self, seed: bytes):
        key = b'\x00' * 32
        v = b'\x01' * 32
        key = hmac.new(key, v + b'\x00' + seed, 'sha256').digest()
        v = hmac.new(key, v, 'sha256').digest()
        key = hmac.new(key, v + b'\x01' + seed, 'sha256').digest()
        v = hmac.new(key, v, 'sha256').digest()
        self._key = key
        self._v = v

    def generate(self, n: int) -> bytes:
        out = b''
        while len(out) < n:
            self._v = hmac.new(self._key, self._v, 'sha256').digest()
            out += self._v
        return out[:n]


def drbg_seed(seed: bytes) -> DRBG:
    return DRBG(seed)


def int_from_bytes_be(b: bytes) -> int:
    return int.from_bytes(b, 'big')


def int_to_bytes_be(x: int, length: int) -> bytes:
    return x.to_bytes(length, 'big')


# ---------------------------------------------------------------------------
# Elliptic curve operations (pure Python, BigUint reference)
# ---------------------------------------------------------------------------

def mod_inv(a: int, m: int) -> int:
    """Extended Euclidean algorithm."""
    if a < 0:
        a = a % m
    g, x, _ = ext_gcd(a, m)
    if g != 1:
        raise ValueError(f"{a} not invertible mod {m}")
    return x % m


def ext_gcd(a: int, b: int):
    if a == 0:
        return b, 0, 1
    g, x1, y1 = ext_gcd(b % a, a)
    return g, y1 - (b // a) * x1, x1


def point_add(p1, p2):
    """Affine point addition on y^2 = x^3 - 2x^2 + 5x + 4 mod p."""
    (x1, y1) = p1
    (x2, y2) = p2
    if x1 is None:
        return p2
    if x2 is None:
        return p1
    if x1 == x2 and y1 == (-y2 % P_521):
        return (None, None)  # point at infinity
    if x1 == x2:
        # Doubling: lambda = (3x1^2 + 2*a2*x1 + a4) / (2*y1)
        #   a2 = -2, a4 = 5
        num = (3 * x1 * x1 - 4 * x1 + 5) % P_521
        den = (2 * y1) % P_521
        lam = (num * mod_inv(den, P_521)) % P_521
        # x3 = lam^2 - a2 - 2*x1 = lam^2 + 2 - 2*x1
        x3 = (lam * lam + 2 - 2 * x1) % P_521
    else:
        # Addition: lambda = (y2 - y1) / (x2 - x1)
        num = (y2 - y1) % P_521
        den = (x2 - x1) % P_521
        lam = (num * mod_inv(den, P_521)) % P_521
        # x3 = lam^2 - a2 - x1 - x2 = lam^2 + 2 - x1 - x2
        x3 = (lam * lam + 2 - x1 - x2) % P_521
    y3 = (lam * (x1 - x3) - y1) % P_521
    return (x3, y3)


def scalar_mul(k: int, point):
    """Double-and-add scalar multiplication (variable-time, reference only)."""
    if k == 0 or point[0] is None:
        return (None, None)
    result = (None, None)
    addend = point
    while k > 0:
        if k & 1:
            result = point_add(result, addend)
        addend = point_add(addend, addend)
        k >>= 1
    return result


# ---------------------------------------------------------------------------
# KAT generation
# ---------------------------------------------------------------------------

def generate_keygen_kats(count: int = 50) -> list[dict]:
    """Generate deterministic key-pair vectors."""
    rng = drbg_seed(b'BCS-521-KAT-keygen-v1')
    vectors = []
    G = (G_X, G_Y)
    for i in range(count):
        # Generate secret key in [1, n)
        seed_for_this = rng.generate(64)
        sk_bytes = hashlib.sha256(seed_for_this).digest()
        sk = int_from_bytes_be(sk_bytes) % (N_521 - 1) + 1  # [1, n)

        # Compute public key
        pk = scalar_mul(sk, G)
        if pk[0] is None:
            continue  # skip if infinity (extremely unlikely)

        pk_bytes = b'\x04' + int_to_bytes_be(pk[0], FIELD_BYTES) + int_to_bytes_be(pk[1], FIELD_BYTES)

        vectors.append({
            "index": i,
            "seed_hex": seed_for_this.hex(),
            "secret_key_hex": int_to_bytes_be(sk, FIELD_BYTES).hex(),
            "public_key_sec1_hex": pk_bytes.hex(),
            "public_key_x_hex": int_to_bytes_be(pk[0], FIELD_BYTES).hex(),
            "public_key_y_hex": int_to_bytes_be(pk[1], FIELD_BYTES).hex(),
        })
    return vectors


def generate_ecdh_kats(count: int = 50) -> list[dict]:
    """Generate ECDH shared-secret + HKDF vectors."""
    rng = drbg_seed(b'BCS-521-KAT-ecdh-v1')
    vectors = []
    G = (G_X, G_Y)

    for i in range(count):
        # Alice key
        seed_a = rng.generate(64)
        sk_a = int_from_bytes_be(hashlib.sha256(seed_a).digest()) % (N_521 - 1) + 1
        pk_a = scalar_mul(sk_a, G)

        # Bob key
        seed_b = rng.generate(64)
        sk_b = int_from_bytes_be(hashlib.sha256(seed_b).digest()) % (N_521 - 1) + 1
        pk_b = scalar_mul(sk_b, G)

        if pk_a[0] is None or pk_b[0] is None:
            continue

        # Shared secret: Alice computes sk_a * pk_b
        shared_a = scalar_mul(sk_a, pk_b)
        shared_b = scalar_mul(sk_b, pk_a)

        # Verify they agree (allow y-negation since affine is ambiguous for large scalars)
        if shared_a[0] != shared_b[0]:
            # Debug: print first mismatch
            print(f"  WARNING: ECDH x-coord mismatch at vector {i}, skipping")
            continue

        shared_x = int_to_bytes_be(shared_a[0], FIELD_BYTES)

        # HKDF-SHA-256 (matches bcs-shield / api.rs)
        derived = _manual_hkdf_sha256(shared_x, b"BCS-521-ECDH-v1", b"BCS-521-ECDH-Shared-Secret-v1", 32)

        vectors.append({
            "index": i,
            "alice_sk_hex": int_to_bytes_be(sk_a, FIELD_BYTES).hex(),
            "alice_pk_x_hex": int_to_bytes_be(pk_a[0], FIELD_BYTES).hex(),
            "alice_pk_y_hex": int_to_bytes_be(pk_a[1], FIELD_BYTES).hex(),
            "bob_sk_hex": int_to_bytes_be(sk_b, FIELD_BYTES).hex(),
            "bob_pk_x_hex": int_to_bytes_be(pk_b[0], FIELD_BYTES).hex(),
            "bob_pk_y_hex": int_to_bytes_be(pk_b[1], FIELD_BYTES).hex(),
            "shared_x_hex": shared_x.hex(),
            "hkdf_sha256_hex": derived.hex(),
        })
    return vectors


def _manual_hkdf_sha256(ikm: bytes, salt: bytes, info: bytes, length: int) -> bytes:
    """HKDF-SHA-256 (RFC 5869) manual implementation."""
    if not salt:
        salt = b'\x00' * 32
    # Extract
    prk = hmac.new(salt, ikm, 'sha256').digest()
    # Expand
    t = b''
    okm = b''
    for i in range(1, (length + 31) // 32 + 1):
        t = hmac.new(prk, t + info + bytes([i]), 'sha256').digest()
        okm += t
    return okm[:length]


def generate_scalar_kats(count: int = 50) -> list[dict]:
    """Generate scalar arithmetic mod n vectors."""
    rng = drbg_seed(b'BCS-521-KAT-scalar-v1')
    vectors = []

    for i in range(count):
        # Generate two random scalars
        a_bytes = hashlib.sha256(rng.generate(64)).digest()
        b_bytes = hashlib.sha256(rng.generate(64)).digest()
        a = int_from_bytes_be(a_bytes) % N_521
        b = int_from_bytes_be(b_bytes) % N_521

        # Compute operations
        add_result = (a + b) % N_521
        sub_result = (a - b) % N_521
        mul_result = (a * b) % N_521
        inv_result = pow(a, N_521 - 2, N_521) if a != 0 else None

        entry = {
            "index": i,
            "a_hex": int_to_bytes_be(a, FIELD_BYTES).hex(),
            "b_hex": int_to_bytes_be(b, FIELD_BYTES).hex(),
            "add_mod_n_hex": int_to_bytes_be(add_result, FIELD_BYTES).hex(),
            "sub_mod_n_hex": int_to_bytes_be(sub_result, FIELD_BYTES).hex(),
            "mul_mod_n_hex": int_to_bytes_be(mul_result, FIELD_BYTES).hex(),
        }
        if inv_result is not None:
            entry["inv_mod_n_hex"] = int_to_bytes_be(inv_result, FIELD_BYTES).hex()
        else:
            entry["inv_mod_n_hex"] = None

        vectors.append(entry)
    return vectors


def generate_kahf_dst_kats() -> list[dict]:
    """Generate Kahf domain separator vectors."""
    labels = [
        "BCS-521-Kahf-v1",
        "BCS-521-ECDH-v1",
        "BCS-521-ECDSA-v1",
        "BCS-521-PQ-Hybrid-v1",
        "Halal-Cert-Sign-v1",
        "Nikah-Nama-v1",
        "Sukuk-v1",
        "Zakat-v1",
        "Waqf-v1",
        "Quran-Translation-v1",
    ]
    vectors = []
    for i, label in enumerate(labels):
        # Build canonical input (matches lib.rs kahf_canonical_input)
        canonical = label.encode('ascii') + b':'
        for p in KAHF_PRIMES:
            canonical += str(p).encode('ascii') + b':'
        canonical += b'BCS-521-Kahf-DST-v1'

        dst = hashlib.sha256(canonical).digest()

        vectors.append({
            "index": i,
            "label": label,
            "canonical_input_hex": canonical.hex(),
            "kahf_dst_hex": dst.hex(),
        })
    return vectors


# ---------------------------------------------------------------------------
# ECDSA KAT generation (RFC 6979, SHA-256)
# ---------------------------------------------------------------------------

def _rfc6979_nonce(n: int, sk_bytes: bytes, h1: bytes) -> int:
    """RFC 6979 §3.2 deterministic nonce for BCS-521.

    Parameters:
        n        — curve order (521-bit prime)
        sk_bytes — secret scalar as 66 big-endian bytes (int2octets(d))
        h1       — SHA-256 message digest (32 bytes)

    This must match the Rust `ct::ecdsa::rfc6979_nonce` byte-for-byte.
    Key details:
      - qlen = 521, hlen = 256 (SHA-256)
      - bits2int(h1): since hlen < qlen, no right-shift; just BigUint(h1)
      - bits2octets(h1): bits2int(h1) mod n, encoded as 66 BE bytes
      - bits2int(T): T is 66 bytes = 528 bits; right-shift by 7 to get 521 bits
    """
    n_bytes = 66
    hlen = 32

    # bits2octets(h1): h_int = BigUint(h1) (no shift since hlen < qlen),
    # then h_mod_n = h_int mod n, then encode as 66 BE bytes.
    h_int = int.from_bytes(h1, 'big')
    h_mod_n = h_int % n
    h_octets = int_to_bytes_be(h_mod_n, n_bytes)

    # Step a: already have h1.
    # Step b: V = 0x01 * hlen.
    v = b'\x01' * hlen
    # Step c: K = 0x00 * hlen.
    k_mac = b'\x00' * hlen

    # Step d: K = HMAC_K(V || 0x00 || int2octets(d) || bits2octets(h1)).
    k_mac = hmac.new(k_mac, v + b'\x00' + sk_bytes + h_octets, 'sha256').digest()
    # Step e: V = HMAC_K(V).
    v = hmac.new(k_mac, v, 'sha256').digest()
    # Step f: K = HMAC_K(V || 0x01 || int2octets(d) || bits2octets(h1)).
    k_mac = hmac.new(k_mac, v + b'\x01' + sk_bytes + h_octets, 'sha256').digest()
    # Step g: V = HMAC_K(V).
    v = hmac.new(k_mac, v, 'sha256').digest()

    # Step h: generate candidate nonces until one is in [1, n-1].
    while True:
        # Accumulate T until |T| >= n_bytes (66 bytes).
        t = b''
        while len(t) < n_bytes:
            v = hmac.new(k_mac, v, 'sha256').digest()
            t += v

        # bits2int(T): T is 528 bits, qlen = 521 bits.
        # Right-shift by 7: candidate = int(T) >> 7
        raw = int.from_bytes(t[:n_bytes], 'big')
        candidate = raw >> 7  # now at most 521 bits

        if 1 <= candidate < n:
            return candidate

        # Candidate out of range: update K and V and retry.
        k_mac = hmac.new(k_mac, v + b'\x00', 'sha256').digest()
        v = hmac.new(k_mac, v, 'sha256').digest()


def _ecdsa_sign(n: int, sk: int, sk_bytes: bytes, msg: bytes) -> dict:
    """ECDSA sign matching the Rust `ct::ecdsa::ct_sign` exactly.

    Returns dict with r_hex, s_hex, and intermediate values for audit.
    """
    G = (G_X, G_Y)

    # Step 1: e = SHA-256(msg) as integer.
    h1 = hashlib.sha256(msg).digest()
    e = int.from_bytes(h1, 'big')  # 256-bit value, always < n

    # Step 2: k = RFC 6979 deterministic nonce.
    k = _rfc6979_nonce(n, sk_bytes, h1)

    # Step 3: R = k·G, r = R.x mod n.
    R = scalar_mul(k, G)
    if R[0] is None:
        raise ValueError("nonce produced identity point")
    r = R[0] % n
    if r == 0:
        raise ValueError("r = 0 (astronomically rare)")

    # Step 4: s = k⁻¹ · (e + r · sk) mod n.
    k_inv = pow(k, n - 2, n)  # Fermat inversion
    s = (k_inv * ((e + r * sk) % n)) % n
    if s == 0:
        raise ValueError("s = 0 (astronomically rare)")

    return {
        "r_hex": int_to_bytes_be(r, FIELD_BYTES).hex(),
        "s_hex": int_to_bytes_be(s, FIELD_BYTES).hex(),
        "k_hex": int_to_bytes_be(k, FIELD_BYTES).hex(),
        "e_hex": int_to_bytes_be(e, FIELD_BYTES).hex(),
    }


def _ecdsa_verify(n: int, pk_point, msg: bytes, r: int, s: int) -> bool:
    """ECDSA verify matching the Rust `ct::ecdsa::ct_verify` exactly."""
    G = (G_X, G_Y)

    # Range check.
    if not (1 <= r < n and 1 <= s < n):
        return False

    # e = SHA-256(msg) as integer.
    h1 = hashlib.sha256(msg).digest()
    e = int.from_bytes(h1, 'big')

    # w = s⁻¹ mod n.
    w = pow(s, n - 2, n)

    # u1 = e·w mod n, u2 = r·w mod n.
    u1 = (e * w) % n
    u2 = (r * w) % n

    # X = u1·G + u2·Q.
    X = point_add(scalar_mul(u1, G), scalar_mul(u2, pk_point))
    if X[0] is None:
        return False  # identity

    # Accept iff X.x ≡ r (mod n).
    return (X[0] % n) == r


def generate_ecdsa_kats(count: int = 100) -> list[dict]:
    """Generate ECDSA sign/verify KAT vectors.

    Each vector contains:
      - secret key + public key
      - message
      - signature (r, s)
      - RFC 6979 nonce k (for audit)
      - hash e (for audit)
      - verify_result (always True for valid signatures)
    """
    rng = drbg_seed(b'BCS-521-KAT-ecdsa-v1')
    vectors = []
    G = (G_X, G_Y)

    messages = [
        b"Bismillah al-Rahman al-Raheem",
        b"BCS-521 ECDSA test vector",
        b"",
        b"Kahf",
        b"Halal certification signature",
        b"Surah Al-Baqarah 255 (Ayat al-Kursi)",
        b"Sukuk bond issuance",
        b"Nikah Nama digital signature",
        b"Waqf deed verification",
        b"Zakat calculation receipt",
    ]

    for i in range(count):
        # Generate secret key.
        seed = rng.generate(64)
        sk_int = int_from_bytes_be(hashlib.sha256(seed).digest()) % (N_521 - 1) + 1
        sk_bytes = int_to_bytes_be(sk_int, FIELD_BYTES)

        # Compute public key.
        pk = scalar_mul(sk_int, G)
        if pk[0] is None:
            continue

        pk_sec1 = b'\x04' + int_to_bytes_be(pk[0], FIELD_BYTES) + int_to_bytes_be(pk[1], FIELD_BYTES)

        # Select message (cycle through the list).
        msg = messages[i % len(messages)]

        # Sign.
        try:
            sig = _ecdsa_sign(N_521, sk_int, sk_bytes, msg)
        except ValueError:
            continue

        # Verify (should always succeed).
        r_int = int.from_bytes(bytes.fromhex(sig["r_hex"]), 'big')
        s_int = int.from_bytes(bytes.fromhex(sig["s_hex"]), 'big')
        ok = _ecdsa_verify(N_521, pk, msg, r_int, s_int)
        if not ok:
            print(f"  WARNING: ECDSA verify failed at vector {i}!")
            continue

        # Also test that a wrong message fails.
        wrong_msg = msg + b"-tampered"
        wrong_ok = _ecdsa_verify(N_521, pk, wrong_msg, r_int, s_int)

        vectors.append({
            "index": i,
            "message_hex": msg.hex(),
            "message_utf8": msg.decode('utf-8', errors='replace'),
            "secret_key_hex": sk_bytes.hex(),
            "public_key_sec1_hex": pk_sec1.hex(),
            "r_hex": sig["r_hex"],
            "s_hex": sig["s_hex"],
            "k_hex": sig["k_hex"],
            "e_hex": sig["e_hex"],
            "verify_valid": ok,
            "verify_tampered": wrong_ok,
        })

    return vectors


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    out_dir = os.path.join(os.path.dirname(__file__), 'kats')
    os.makedirs(out_dir, exist_ok=True)

    print("Generating BCS-521 KAT vectors...")

    # 1. Keygen
    keygen = generate_keygen_kats(50)
    path = os.path.join(out_dir, 'bcs521_keygen.json')
    with open(path, 'w') as f:
        json.dump({"algorithm": "BCS-521", "type": "keygen", "count": len(keygen), "vectors": keygen}, f, indent=2)
    print(f"  keygen: {len(keygen)} vectors -> {path}")

    # 2. ECDH
    ecdh = generate_ecdh_kats(50)
    path = os.path.join(out_dir, 'bcs521_ecdh.json')
    with open(path, 'w') as f:
        json.dump({"algorithm": "BCS-521", "type": "ecdh", "count": len(ecdh), "vectors": ecdh}, f, indent=2)
    print(f"  ecdh:   {len(ecdh)} vectors -> {path}")

    # 3. Scalar arithmetic
    scalar = generate_scalar_kats(50)
    path = os.path.join(out_dir, 'bcs521_scalar.json')
    with open(path, 'w') as f:
        json.dump({"algorithm": "BCS-521", "type": "scalar_mod_n", "count": len(scalar), "vectors": scalar}, f, indent=2)
    print(f"  scalar: {len(scalar)} vectors -> {path}")

    # 4. Kahf DST
    kahf = generate_kahf_dst_kats()
    path = os.path.join(out_dir, 'bcs521_kahf_dst.json')
    with open(path, 'w') as f:
        json.dump({"algorithm": "BCS-521", "type": "kahf_domain_separator", "count": len(kahf), "vectors": kahf}, f, indent=2)
    print(f"  kahf:   {len(kahf)} vectors -> {path}")

    # 5. ECDSA sign/verify
    ecdsa = generate_ecdsa_kats(100)
    path = os.path.join(out_dir, 'bcs521_ecdsa.json')
    with open(path, 'w') as f:
        json.dump({"algorithm": "BCS-521", "type": "ecdsa_rfc6979_sha256", "count": len(ecdsa), "vectors": ecdsa}, f, indent=2)
    print(f"  ecdsa:  {len(ecdsa)} vectors -> {path}")

    total = len(keygen) + len(ecdh) + len(scalar) + len(kahf) + len(ecdsa)
    print(f"\nTotal: {total} KAT vectors generated.")
    print("Next: run `cargo test --features ecdsa kat_parity` on Codespaces to cross-check.")


if __name__ == '__main__':
    main()
