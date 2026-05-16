\\ BCS-521 Independent Cardinality Proof - Pari/GP v5 (Colab-safe)
\\ Run: gp -q bcs521_pari_proof.gp

default(parisize, "1G");

p_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363;
n_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231;
E = ellinit([0, -2, 0, 5, 4], p_521);

fname = "bcs521_pari_proof_result.txt";
out = "";
record(s) = { print(s); out = concat(out, concat(s, "\n")); };

record("=========================================================================");
record("BCS-521 Independent Cardinality Proof  (Pari/GP v5)");
record(Str("Timestamp ms: ", getwalltime()));
record("=========================================================================");
record(Str("p bits = ", #binary(p_521)));
record(Str("n bits = ", #binary(n_521)));
record("");

record(">>> (1) Proving p is prime (APR-CL).");
t0 = getwalltime(); ok_p = isprime(p_521, 2); dt = (getwalltime() - t0) / 1000.0;
record(Str("    p prime: ", ok_p, "  [", dt, " s]"));
record("");

record(">>> (2) Proving n is prime (APR-CL).");
t0 = getwalltime(); ok_n = isprime(n_521, 2); dt = (getwalltime() - t0) / 1000.0;
record(Str("    n prime: ", ok_n, "  [", dt, " s]"));
record("");

record(">>> (3) Computing #E(F_p) via SEA.");
t0 = getwalltime(); ap_computed = ellap(E); card = p_521 + 1 - ap_computed; dt = (getwalltime() - t0) / 1000.0;
record(Str("    ap = ", ap_computed));
record(Str("    card == n: ", card == n_521));
record(Str("    time: ", dt, " s"));
record("");

record(">>> (4) G=(0,2) on curve.");
on_curve = ellisoncurve(E, [0, 2]);
record(Str("    on curve: ", on_curve));
record("");

record(">>> (5) n*G = O.");
nG = ellmul(E, [0, 2], n_521); is_inf = (nG == [0]);
record(Str("    n*G=[0]: ", is_inf));
record("");

record(">>> (6) Cofactor h.");
h = card / n_521;
record(Str("    h=", h, "  h==1: ", h == 1));
record("");

record(">>> (7) Hasse bound.");
frob_t = p_521 + 1 - card; hasse_ok = (frob_t^2 <= 4*p_521);
record(Str("    Hasse OK: ", hasse_ok));
record("");

record(">>> (8) Not anomalous.");
record(Str("    p!=n: ", p_521 != n_521));
record("");

record(">>> (9) MOV safety (embedding degree > 100).");
t0 = getwalltime(); mov_safe = 1; pm = Mod(p_521, n_521); pk = pm;
for(k = 1, 100, if(pk == Mod(1, n_521), mov_safe = 0); pk = pk * pm);
dt = (getwalltime() - t0) / 1000.0;
record(Str("    MOV safe (k>100): ", mov_safe, "  [", dt, " s]"));
record("");

record(">>> (10) Twist order.");
tw = 2*(p_521 + 1) - card;
record(Str("    twist bits = ", #binary(tw)));
record("");

record("=========================================================================");
record("FINAL VERDICT");
record("=========================================================================");
crit = ok_p && ok_n && (card == n_521) && on_curve && is_inf && (h == 1) && hasse_ok && (p_521 != n_521) && mov_safe;
record(Str("All critical: ", crit));
if(crit, record("VERDICT: PASS - BCS-521 independently certified by Pari/GP."), record("VERDICT: FAIL - re-investigate."));

write(fname, out);
record(Str("Saved: ", fname));
quit;
