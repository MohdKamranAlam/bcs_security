# ===============================================================
# BCS-521 FROZEN TEST VECTORS — single-cell generator
# ===============================================================
#
# Run this ONE cell in Kaggle / Colab (or local Python). It generates
# deterministic test vectors that future implementations (Rust, Go, JS, ...)
# MUST reproduce exactly. This locks down the curve behaviour permanently.
#
# Output:  bcs521_test_vectors.json  (downloadable / can be saved into the repo)
#
# Contents:
#   - curve parameters
#   - 5 deterministic scalar multiplications k * G  for known k
#   - 1 deterministic ECDH session (Alice + Bob) with fixed seeds
#   - HKDF-SHA-512 derived key
#   - AES-256-GCM encryption of a known plaintext with deterministic nonce
#   - All values hex-encoded for cross-language verification
#
# Dependencies (pre-installed on Kaggle/Colab): hashlib, hmac, secrets
#   pycryptodome (for AES-GCM) — installed via pip if missing.
# ===============================================================

import json, hashlib, hmac, sys, subprocess
from datetime import datetime, timezone

# ---------- pycryptodome for AES-256-GCM ----------
try:
    from Crypto.Cipher import AES
except ImportError:
    print("Installing pycryptodome ...")
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "pycryptodome"], check=True)
    from Crypto.Cipher import AES

# ---------- BCS-521 frozen parameters ----------
p = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
n = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231
A2, A4, A6 = -2, 5, 4    # y^2 = x^3 + A2*x^2 + A4*x + A6
Gx, Gy = 0, 2
COORD_LEN = 66           # 521 bits -> 66 bytes (ceil(521/8))

# ---------- EC arithmetic (pure Python, independent reference) ----------
def inv(a, m): return pow(a, -1, m)

def on_curve(P):
    if P is None: return True
    x, y = P
    return (y*y) % p == (x*x*x + A2*x*x + A4*x + A6) % p

def add(P, Q):
    if P is None: return Q
    if Q is None: return P
    x1, y1 = P; x2, y2 = Q
    if x1 == x2:
        if (y1 + y2) % p == 0: return None
        m = ((3*x1*x1 + 2*A2*x1 + A4) * inv(2*y1 % p, p)) % p
    else:
        m = ((y2 - y1) * inv((x2 - x1) % p, p)) % p
    x3 = (m*m - A2 - x1 - x2) % p
    y3 = (m * (x1 - x3) - y1) % p
    return (x3, y3)

def smul(k, P):
    R = None; Q = P
    while k > 0:
        if k & 1: R = add(R, Q)
        Q = add(Q, Q); k >>= 1
    return R

# ---------- Deterministic key derivation from a label ----------
def derive_sk(label: str) -> int:
    """Deterministic private key in [1, n-1] from a fixed label."""
    seed = hashlib.sha512(b"BCS-521-sk-v1::" + label.encode()).digest()
    # Expand to 128 bytes to remove any modular bias for n<2^521
    expanded = seed + hashlib.sha512(b"expand::" + seed).digest()
    return (int.from_bytes(expanded, "big") % (n - 1)) + 1

def point_to_hex(P):
    if P is None: return {"infinity": True}
    x, y = P
    return {
        "x_hex": x.to_bytes(COORD_LEN, "big").hex(),
        "y_hex": y.to_bytes(COORD_LEN, "big").hex(),
    }

# ---------- HKDF-SHA-512 (RFC 5869) ----------
def hkdf_extract(salt: bytes, ikm: bytes) -> bytes:
    return hmac.new(salt, ikm, hashlib.sha512).digest()

def hkdf_expand(prk: bytes, info: bytes, length: int) -> bytes:
    out = b""; t = b""; counter = 1
    while len(out) < length:
        t = hmac.new(prk, t + info + bytes([counter]), hashlib.sha512).digest()
        out += t; counter += 1
    return out[:length]

def hkdf(salt: bytes, ikm: bytes, info: bytes, length: int = 64) -> bytes:
    return hkdf_expand(hkdf_extract(salt, ikm), info, length)

# ---------- Kahf Domain Separator (Theorem 18 binding) ----------
# Five sacred Kahf primes in canonical alphabetical key order.
# This MUST byte-match Rust `bcs_core_rust::KAHF_PRIMES`.
KAHF_PRIMES = [
    ("p_kahf_first_decimal", 2141),
    ("p_kahf_last_zf",       2969),
    ("p_kahf_sleepers",      7),
    ("p_kahf_surah_zf",      19),
    ("p_kahf_years_zf",      373),
]

def kahf_canonical_input(label: str) -> bytes:
    parts = [label.encode() + b":"]
    for k, v in KAHF_PRIMES:
        parts.append(f"{k}={v};".encode())
    return b"".join(parts)

def kahf_dst(label: str) -> bytes:
    """32-byte SHA-256 of the canonical Kahf input."""
    return hashlib.sha256(kahf_canonical_input(label)).digest()

def hkdf_kahf(salt: bytes, ikm: bytes, info: bytes,
              dst_label: str = "BCS-521-Kahf-v1", length: int = 64) -> bytes:
    """Kahf-bound HKDF: prepend DST(label) || '|' to user info."""
    bound_info = kahf_dst(dst_label) + b"|" + info
    return hkdf(salt, ikm, bound_info, length)

# ===============================================================
# Build vectors
# ===============================================================
print("=" * 72)
print("BCS-521  FROZEN TEST VECTORS GENERATOR")
print(datetime.now(timezone.utc).isoformat())
print("=" * 72)

G = (Gx, Gy)
assert on_curve(G), "G not on curve - parameters wrong!"

# ---- (1) Scalar multiplication vectors ----
print("\n[1] Scalar-mul vectors:")
scalar_mul = []
for k in [1, 2, 3, 1000, n - 1]:
    P = smul(k, G)
    label = "n-1" if k == n - 1 else str(k)
    print(f"  k={label}:  P = ({hex(P[0])[:24]}..., {hex(P[1])[:24]}...)")
    assert on_curve(P)
    scalar_mul.append({
        "k_label": label,
        "k_hex": (k).to_bytes(COORD_LEN, "big").hex(),
        "point": point_to_hex(P),
    })

# (n-1)*G + G should be O
P_nminus1 = smul(n - 1, G)
P_sum = add(P_nminus1, G)
assert P_sum is None, "(n-1)G + G must be infinity!"
print("  (n-1)*G + G = O   [OK]")

# n * G = O
assert smul(n, G) is None, "n*G must be infinity!"
print("  n*G = O           [OK]")

# ---- (2) ECDH session vectors ----
print("\n[2] ECDH session  (Alice + Bob, deterministic seeds):")

alice_label = "BCS-521 Bismillah Alice v1"
bob_label   = "BCS-521 Bismillah Bob v1"

alice_sk = derive_sk(alice_label)
bob_sk   = derive_sk(bob_label)

alice_pk = smul(alice_sk, G)
bob_pk   = smul(bob_sk,   G)
assert on_curve(alice_pk) and on_curve(bob_pk)

# ECDH
shared_alice = smul(alice_sk, bob_pk)
shared_bob   = smul(bob_sk,   alice_pk)
assert shared_alice == shared_bob, "ECDH disagreement!"
assert shared_alice is not None

shared_x = shared_alice[0].to_bytes(COORD_LEN, "big")
print(f"  alice_sk  = {hex(alice_sk)[:40]}...")
print(f"  bob_sk    = {hex(bob_sk)[:40]}...")
print(f"  shared_x  = {shared_x.hex()[:40]}...  ({len(shared_x)} bytes)")

# ---- (3) HKDF-SHA-512 derivation ----
print("\n[3] HKDF-SHA-512:")
salt = b"BCS-521-salt-v1"
info = b"BCS-521-ECDH-AES256GCM-v1"
hkdf_out_64 = hkdf(salt, shared_x, info, 64)
aes_key   = hkdf_out_64[:32]   # 256-bit AES key
aes_nonce = hkdf_out_64[32:32+12]  # 96-bit GCM nonce (deterministic from shared)
print(f"  hkdf(64 B) = {hkdf_out_64.hex()[:40]}...")
print(f"  AES key    = {aes_key.hex()}")
print(f"  GCM nonce  = {aes_nonce.hex()}")

# ---- (4) AES-256-GCM encryption ----
print("\n[4] AES-256-GCM:")
plaintext = b"Bismillah ar-Rahman ar-Raheem -- BCS-521 test vector v1"
aad       = b"BCS-521-AAD-v1"
cipher = AES.new(aes_key, AES.MODE_GCM, nonce=aes_nonce)
cipher.update(aad)
ciphertext, tag = cipher.encrypt_and_digest(plaintext)
print(f"  plaintext  = {plaintext!r}")
print(f"  aad        = {aad!r}")
print(f"  ciphertext = {ciphertext.hex()}")
print(f"  tag        = {tag.hex()}")

# Decrypt sanity check
dec = AES.new(aes_key, AES.MODE_GCM, nonce=aes_nonce)
dec.update(aad)
recovered = dec.decrypt_and_verify(ciphertext, tag)
assert recovered == plaintext, "AES-GCM round-trip failed!"
print("  decrypt round-trip [OK]")

# ---- (5) Kahf-bound HKDF + AES-256-GCM (Theorem 18) ----
print("\n[5] Kahf-bound HKDF + AES-256-GCM:")
kahf_dst_521 = kahf_dst("BCS-521-Kahf-v1")
print(f"  Kahf DST     = {kahf_dst_521.hex()}")
hkdf_kahf_64  = hkdf_kahf(salt, shared_x, info, "BCS-521-Kahf-v1", 64)
aes_key_kahf  = hkdf_kahf_64[:32]
nonce_kahf    = hkdf_kahf_64[32:32+12]
print(f"  hkdf_kahf 64 = {hkdf_kahf_64.hex()[:40]}...")
print(f"  AES key      = {aes_key_kahf.hex()}")
print(f"  GCM nonce    = {nonce_kahf.hex()}")

cipher_k = AES.new(aes_key_kahf, AES.MODE_GCM, nonce=nonce_kahf)
cipher_k.update(aad)
ct_kahf, tag_kahf = cipher_k.encrypt_and_digest(plaintext)
print(f"  ciphertext   = {ct_kahf.hex()}")
print(f"  tag          = {tag_kahf.hex()}")

dec_k = AES.new(aes_key_kahf, AES.MODE_GCM, nonce=nonce_kahf)
dec_k.update(aad)
assert dec_k.decrypt_and_verify(ct_kahf, tag_kahf) == plaintext
assert ct_kahf != ciphertext, "Kahf binding MUST change ciphertext"
print("  Kahf-bound differs from plain [OK]")

# ===============================================================
# Assemble frozen JSON
# ===============================================================
out = {
    "version":   "BCS-521 test vectors v1.0",
    "frozen_at": datetime.now(timezone.utc).isoformat(),
    "curve": {
        "name":     "BCS-521",
        "equation": "y^2 = x^3 - 2x^2 + 5x + 4",
        "a2": A2, "a4": A4, "a6": A6,
        "p_hex": p.to_bytes(COORD_LEN, "big").hex(),
        "n_hex": n.to_bytes(COORD_LEN, "big").hex(),
        "cofactor_h": 1,
        "G": point_to_hex(G),
        "coord_len_bytes": COORD_LEN,
    },
    "scalar_mul":  scalar_mul,
    "ecdh": {
        "alice_label": alice_label,
        "bob_label":   bob_label,
        "alice_sk_hex": alice_sk.to_bytes(COORD_LEN, "big").hex(),
        "bob_sk_hex":   bob_sk.to_bytes(COORD_LEN, "big").hex(),
        "alice_pk":    point_to_hex(alice_pk),
        "bob_pk":      point_to_hex(bob_pk),
        "shared_point": point_to_hex(shared_alice),
        "shared_x_hex": shared_x.hex(),
    },
    "hkdf": {
        "hash":     "SHA-512",
        "salt_ascii": "BCS-521-salt-v1",
        "info_ascii": "BCS-521-ECDH-AES256GCM-v1",
        "salt_hex": salt.hex(),
        "info_hex": info.hex(),
        "ikm_hex":  shared_x.hex(),
        "out_len":  64,
        "out_hex":  hkdf_out_64.hex(),
    },
    "aes_256_gcm": {
        "key_hex":        aes_key.hex(),
        "nonce_hex":      aes_nonce.hex(),
        "aad_ascii":      "BCS-521-AAD-v1",
        "aad_hex":        aad.hex(),
        "plaintext_ascii": plaintext.decode(),
        "plaintext_hex":  plaintext.hex(),
        "ciphertext_hex": ciphertext.hex(),
        "tag_hex":        tag.hex(),
    },
    "kahf_binding": {
        "dst_label":     "BCS-521-Kahf-v1",
        "dst_hex":       kahf_dst_521.hex(),
        "primes":        {k: v for k, v in KAHF_PRIMES},
        "canonical_input_ascii": kahf_canonical_input("BCS-521-Kahf-v1").decode(),
        "canonical_input_hex":   kahf_canonical_input("BCS-521-Kahf-v1").hex(),
        "hkdf_kahf_64_hex":      hkdf_kahf_64.hex(),
        "aes_key_hex":           aes_key_kahf.hex(),
        "nonce_hex":             nonce_kahf.hex(),
        "ciphertext_hex":        ct_kahf.hex(),
        "tag_hex":               tag_kahf.hex(),
        "note": "Kahf-bound HKDF info = DST(label) || 0x7C || user_info. AES key + nonce derived from this Kahf-bound HKDF output.",
    },
    "negative_vectors": {
        "off_curve_point": {
            "description": "Point with valid coords but not on E. Implementations MUST reject.",
            "x_hex": (1).to_bytes(COORD_LEN, "big").hex(),
            "y_hex": (1).to_bytes(COORD_LEN, "big").hex(),
        },
        "identity_as_pk": {
            "description": "Point at infinity as a public key. Implementations MUST reject.",
            "infinity": True,
        },
    },
    "interop_notes": [
        "All coordinates are 66-byte (528-bit) big-endian, top 7 bits are zero.",
        "Private keys are in [1, n-1], encoded as 66-byte big-endian.",
        "ECDH shared secret is the X-coordinate of (alice_sk * bob_pk) as 66 bytes.",
        "HKDF input keying material (IKM) = shared_x (66 bytes).",
        "AES key = first 32 bytes of HKDF output; GCM nonce = next 12 bytes.",
        "This is a research/reference test vector. Production use requires constant-time impl.",
    ],
}

with open("bcs521_test_vectors.json", "w") as fh:
    json.dump(out, fh, indent=2)

print("\n" + "=" * 72)
print("FROZEN bcs521_test_vectors.json written.")
print(f"Total scalar-mul vectors:  {len(scalar_mul)}")
print(f"ECDH:                      Alice+Bob,  shared {len(shared_x)} B")
print(f"AES-256-GCM tag:           {tag.hex()}")
print("=" * 72)

# Auto-download in Colab
try:
    from google.colab import files
    files.download("bcs521_test_vectors.json")
except ImportError:
    print("\n(Kaggle: download bcs521_test_vectors.json from right sidebar 'Output')")
