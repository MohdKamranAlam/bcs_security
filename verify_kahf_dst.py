"""BCS Kahf DST — Python verifier. Output MUST match the Rust binary."""
import hashlib

KAHF_PRIMES = {
    "p_kahf_first_decimal": 2141,
    "p_kahf_last_zf":       2969,
    "p_kahf_sleepers":      7,
    "p_kahf_surah_zf":      19,
    "p_kahf_years_zf":      373,
}

def canonical_input(label):
    out = label.encode("utf-8") + b":"
    for k in sorted(KAHF_PRIMES):
        out += k.encode("utf-8") + b"="
        out += str(KAHF_PRIMES[k]).encode("utf-8") + b";"
    return out

def kahf_dst(label):
    return hashlib.sha256(canonical_input(label)).digest()

def show(label):
    raw = canonical_input(label)
    dst = kahf_dst(label)
    print(f"\nLabel: {label}")
    print(f"  canonical_input ({len(raw)} bytes):")
    print(f"    ASCII : {raw.decode('utf-8')}")
    print(f"    HEX   : {raw.hex()}")
    print(f"  DST (SHA-256, 32 bytes):")
    print(f"    HEX   : {dst.hex()}")

if __name__ == "__main__":
    print("=" * 72)
    print("BCS Kahf Domain Separator (Python verifier)")
    print("=" * 72)
    print("\n[1] Sacred Kahf primes (canonical alphabetical order):")
    for k in sorted(KAHF_PRIMES):
        print(f"    {k:<24} = {KAHF_PRIMES[k]}")
    print("\n[2] DST values:")
    for lbl in ["BCS-Kahf-v1", "BCS-256-Kahf-v1", "BCS-521-Kahf-v1"]:
        show(lbl)
    print("\n" + "=" * 72)
    print("Compare HEX values with Rust output — they MUST be identical.")
    print("=" * 72)
